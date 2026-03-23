import React, { useState, useMemo, useCallback } from 'react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import { Table2, BarChart3, Undo2, Redo2, RotateCcw } from 'lucide-react';

// ---------------------------------------------------------------------------
// Mock data — 8x12 Boost Pressure map
// ---------------------------------------------------------------------------
const X_AXIS: number[] = [800, 1000, 1200, 1500, 1800, 2000, 2200, 2500, 2800, 3000, 3200, 3500];
const Y_AXIS: number[] = [10, 20, 30, 40, 50, 60, 70, 80];
const UNIT = 'mbar';
const MAP_NAME = 'Boost Pressure';
const ECU_TYPE = 'EDC17C46';

function generateBoostMap(): number[][] {
  return Y_AXIS.map((load, ri) =>
    X_AXIS.map((rpm, ci) => {
      const base = 1000 + (rpm / 3500) * 1400 + (load / 80) * 600;
      const noise = Math.sin(ri * 3 + ci * 7) * 60;
      return Math.round(base + noise);
    }),
  );
}

function cloneMap(m: number[][]): number[][] {
  return m.map((r) => [...r]);
}

// ---------------------------------------------------------------------------
// Value-to-color gradient: blue -> green -> yellow -> red
// ---------------------------------------------------------------------------
function valueColor(val: number, min: number, max: number): string {
  const t = Math.max(0, Math.min(1, (val - min) / (max - min || 1)));
  // 0=blue, 0.33=cyan, 0.5=green, 0.66=yellow, 1=red
  if (t < 0.33) {
    const s = t / 0.33;
    const r = 30;
    const g = Math.round(80 + s * 120);
    const b = Math.round(220 - s * 100);
    return `rgb(${r},${g},${b})`;
  }
  if (t < 0.66) {
    const s = (t - 0.33) / 0.33;
    const r = Math.round(30 + s * 200);
    const g = Math.round(200 - s * 20);
    const b = Math.round(120 - s * 80);
    return `rgb(${r},${g},${b})`;
  }
  const s = (t - 0.66) / 0.34;
  const r = Math.round(230 + s * 25);
  const g = Math.round(180 - s * 140);
  const b = Math.round(40 - s * 30);
  return `rgb(${r},${g},${b})`;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export default function MapEditor2D() {
  const stockMap = useMemo(() => generateBoostMap(), []);
  const [mapData, setMapData] = useState<number[][]>(() => cloneMap(stockMap));
  const [history, setHistory] = useState<number[][][]>([]);
  const [redoStack, setRedoStack] = useState<number[][][]>([]);

  const [selectedCell, setSelectedCell] = useState<{ row: number; col: number } | null>(null);
  const [editingCell, setEditingCell] = useState<{ row: number; col: number } | null>(null);
  const [editValue, setEditValue] = useState('');
  const [chartMode, setChartMode] = useState<'row' | 'col'>('row');

  // Global min/max for color scale
  const { min: globalMin, max: globalMax } = useMemo(() => {
    let min = Infinity;
    let max = -Infinity;
    for (const row of mapData) {
      for (const v of row) {
        if (v < min) min = v;
        if (v > max) max = v;
      }
    }
    return { min, max };
  }, [mapData]);

  // ---------- edit helpers ----------
  const pushHistory = useCallback(() => {
    setHistory((prev) => [...prev.slice(-50), cloneMap(mapData)]);
    setRedoStack([]);
  }, [mapData]);

  const commitEdit = useCallback(
    (row: number, col: number, raw: string) => {
      const num = parseInt(raw, 10);
      if (isNaN(num)) {
        setEditingCell(null);
        return;
      }
      pushHistory();
      setMapData((prev) => {
        const next = cloneMap(prev);
        next[row][col] = Math.max(0, Math.min(9999, num));
        return next;
      });
      setEditingCell(null);
    },
    [pushHistory],
  );

  const undo = useCallback(() => {
    if (history.length === 0) return;
    const prev = history[history.length - 1];
    setRedoStack((r) => [...r, cloneMap(mapData)]);
    setMapData(prev);
    setHistory((h) => h.slice(0, -1));
  }, [history, mapData]);

  const redo = useCallback(() => {
    if (redoStack.length === 0) return;
    const next = redoStack[redoStack.length - 1];
    setHistory((h) => [...h, cloneMap(mapData)]);
    setMapData(next);
    setRedoStack((r) => r.slice(0, -1));
  }, [redoStack, mapData]);

  const resetMap = useCallback(() => {
    pushHistory();
    setMapData(cloneMap(stockMap));
  }, [pushHistory, stockMap]);

  // ---------- chart data ----------
  const chartData = useMemo(() => {
    if (!selectedCell) return [];
    if (chartMode === 'row') {
      return X_AXIS.map((rpm, ci) => ({
        label: `${rpm}`,
        value: mapData[selectedCell.row][ci],
        stock: stockMap[selectedCell.row][ci],
      }));
    }
    return Y_AXIS.map((load, ri) => ({
      label: `${load}`,
      value: mapData[ri][selectedCell.col],
      stock: stockMap[ri][selectedCell.col],
    }));
  }, [selectedCell, chartMode, mapData, stockMap]);

  // ---------- delta from stock ----------
  const delta = useMemo(() => {
    if (!selectedCell) return null;
    const cur = mapData[selectedCell.row][selectedCell.col];
    const stk = stockMap[selectedCell.row][selectedCell.col];
    return cur - stk;
  }, [selectedCell, mapData, stockMap]);

  return (
    <div className="flex flex-col h-full bg-zinc-900 text-zinc-100 text-sm rounded-lg border border-zinc-700 overflow-hidden">
      {/* ---- Header bar ---- */}
      <div className="flex items-center gap-3 px-3 py-2 bg-zinc-800 border-b border-zinc-700 flex-wrap">
        <Table2 size={16} className="text-sky-400" />
        <span className="font-semibold text-zinc-100">{MAP_NAME}</span>
        <span className="text-zinc-500">|</span>
        <span className="text-zinc-400">{Y_AXIS.length}&times;{X_AXIS.length}</span>
        <span className="text-zinc-500">|</span>
        <span className="text-zinc-400">{UNIT}</span>
        <span className="text-zinc-500">|</span>
        <span className="text-zinc-400">{ECU_TYPE}</span>

        <div className="ml-auto flex items-center gap-1">
          <button
            className="p-1.5 rounded hover:bg-zinc-700 disabled:opacity-30"
            onClick={undo}
            disabled={history.length === 0}
            title="Undo"
          >
            <Undo2 size={14} />
          </button>
          <button
            className="p-1.5 rounded hover:bg-zinc-700 disabled:opacity-30"
            onClick={redo}
            disabled={redoStack.length === 0}
            title="Redo"
          >
            <Redo2 size={14} />
          </button>
          <button
            className="p-1.5 rounded hover:bg-zinc-700"
            onClick={resetMap}
            title="Reset to stock"
          >
            <RotateCcw size={14} />
          </button>
          <div className="w-px h-5 bg-zinc-600 mx-1" />
          <button
            className={`px-2 py-1 rounded text-xs ${chartMode === 'row' ? 'bg-sky-600 text-white' : 'bg-zinc-700 text-zinc-300 hover:bg-zinc-600'}`}
            onClick={() => setChartMode('row')}
          >
            Row
          </button>
          <button
            className={`px-2 py-1 rounded text-xs ${chartMode === 'col' ? 'bg-sky-600 text-white' : 'bg-zinc-700 text-zinc-300 hover:bg-zinc-600'}`}
            onClick={() => setChartMode('col')}
          >
            Col
          </button>
        </div>
      </div>

      {/* ---- Main area: table + chart ---- */}
      <div className="flex flex-1 overflow-hidden">
        {/* ---- Table ---- */}
        <div className="flex-1 overflow-auto p-2">
          <table className="border-collapse">
            <thead>
              <tr>
                <th className="sticky top-0 left-0 z-20 bg-zinc-800 px-2 py-1 text-[11px] text-zinc-500 border border-zinc-700">
                  RPM&rarr;<br />Load&darr;
                </th>
                {X_AXIS.map((rpm) => (
                  <th
                    key={rpm}
                    className="sticky top-0 z-10 bg-zinc-800 px-1 py-1 text-[11px] text-amber-400 border border-zinc-700 font-normal whitespace-nowrap"
                  >
                    {rpm}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {Y_AXIS.map((load, ri) => (
                <tr key={load}>
                  <td className="sticky left-0 z-10 bg-zinc-800 px-2 py-0.5 text-[11px] text-amber-400 border border-zinc-700 whitespace-nowrap font-normal">
                    {load}
                  </td>
                  {X_AXIS.map((_rpm, ci) => {
                    const val = mapData[ri][ci];
                    const isSelected = selectedCell?.row === ri && selectedCell?.col === ci;
                    const isEditing = editingCell?.row === ri && editingCell?.col === ci;
                    const diff = val - stockMap[ri][ci];

                    return (
                      <td
                        key={ci}
                        className={`border border-zinc-700/60 px-0.5 py-0 text-center cursor-pointer select-none transition-colors ${
                          isSelected ? 'ring-2 ring-sky-400 ring-inset' : ''
                        }`}
                        style={{ backgroundColor: valueColor(val, globalMin, globalMax) }}
                        onClick={() => {
                          setSelectedCell({ row: ri, col: ci });
                          if (!isEditing) setEditingCell(null);
                        }}
                        onDoubleClick={() => {
                          setEditingCell({ row: ri, col: ci });
                          setEditValue(String(val));
                        }}
                      >
                        {isEditing ? (
                          <input
                            autoFocus
                            className="w-14 bg-zinc-900/90 text-zinc-100 text-center text-xs outline-none rounded px-0.5"
                            value={editValue}
                            onChange={(e) => setEditValue(e.target.value)}
                            onBlur={() => commitEdit(ri, ci, editValue)}
                            onKeyDown={(e) => {
                              if (e.key === 'Enter') commitEdit(ri, ci, editValue);
                              if (e.key === 'Escape') setEditingCell(null);
                            }}
                          />
                        ) : (
                          <span className="text-xs font-mono text-zinc-100 drop-shadow-[0_1px_1px_rgba(0,0,0,0.8)]">
                            {val}
                            {diff !== 0 && (
                              <span className={`ml-0.5 text-[9px] ${diff > 0 ? 'text-red-300' : 'text-blue-300'}`}>
                                {diff > 0 ? '+' : ''}{diff}
                              </span>
                            )}
                          </span>
                        )}
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {/* ---- Chart panel ---- */}
        <div className="w-[340px] shrink-0 border-l border-zinc-700 bg-zinc-800/40 p-3 flex flex-col">
          <div className="flex items-center gap-2 mb-2 text-xs text-zinc-400">
            <BarChart3 size={14} className="text-sky-400" />
            {selectedCell ? (
              <span>
                {chartMode === 'row'
                  ? `Row: Load = ${Y_AXIS[selectedCell.row]} mg/stroke`
                  : `Col: RPM = ${X_AXIS[selectedCell.col]}`}
              </span>
            ) : (
              <span>Select a cell to view chart</span>
            )}
          </div>

          {selectedCell && chartData.length > 0 ? (
            <div className="flex-1 min-h-0">
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={chartData} margin={{ top: 8, right: 8, bottom: 4, left: -8 }}>
                  <CartesianGrid strokeDasharray="3 3" stroke="#3f3f46" />
                  <XAxis
                    dataKey="label"
                    tick={{ fill: '#a1a1aa', fontSize: 10 }}
                    label={{
                      value: chartMode === 'row' ? 'RPM' : 'Load (mg)',
                      position: 'insideBottom',
                      offset: -2,
                      fill: '#71717a',
                      fontSize: 10,
                    }}
                  />
                  <YAxis
                    tick={{ fill: '#a1a1aa', fontSize: 10 }}
                    label={{
                      value: UNIT,
                      angle: -90,
                      position: 'insideLeft',
                      offset: 16,
                      fill: '#71717a',
                      fontSize: 10,
                    }}
                  />
                  <Tooltip
                    contentStyle={{ backgroundColor: '#18181b', border: '1px solid #3f3f46', borderRadius: 6, fontSize: 12 }}
                    labelStyle={{ color: '#a1a1aa' }}
                  />
                  <Line
                    type="monotone"
                    dataKey="stock"
                    stroke="#6b7280"
                    strokeDasharray="4 4"
                    dot={false}
                    name="Stock"
                  />
                  <Line
                    type="monotone"
                    dataKey="value"
                    stroke="#38bdf8"
                    strokeWidth={2}
                    dot={{ r: 3, fill: '#38bdf8' }}
                    activeDot={{ r: 5 }}
                    name="Modified"
                  />
                </LineChart>
              </ResponsiveContainer>
            </div>
          ) : (
            <div className="flex-1 flex items-center justify-center text-zinc-600 text-xs">
              No cell selected
            </div>
          )}
        </div>
      </div>

      {/* ---- Status bar ---- */}
      <div className="flex items-center justify-between px-3 py-1.5 bg-zinc-800 border-t border-zinc-700 text-xs text-zinc-400">
        {selectedCell ? (
          <span>
            Cell [{selectedCell.row},{selectedCell.col}] ={' '}
            <span className="text-sky-400">{mapData[selectedCell.row][selectedCell.col]} {UNIT}</span>
            {delta !== null && delta !== 0 && (
              <span className={`ml-2 ${delta > 0 ? 'text-red-400' : 'text-blue-400'}`}>
                {'\u0394'} from stock: {delta > 0 ? '+' : ''}{delta}
              </span>
            )}
            <span className="ml-3 text-zinc-500">
              RPM={X_AXIS[selectedCell.col]}, Load={Y_AXIS[selectedCell.row]} mg/stroke
            </span>
          </span>
        ) : (
          <span>Click a cell to select, double-click to edit</span>
        )}
        <span>
          History: {history.length} | Edits: {
            mapData.reduce(
              (acc, row, ri) => acc + row.reduce((a, v, ci) => a + (v !== stockMap[ri][ci] ? 1 : 0), 0),
              0,
            )
          } cells modified
        </span>
      </div>
    </div>
  );
}
