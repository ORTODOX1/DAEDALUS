import { useState, useEffect, useRef, useCallback } from "react";
import { Play, Square, Circle } from "lucide-react";
import { LineChart, Line, ResponsiveContainer, YAxis } from "recharts";

interface GaugeConfig {
  id: string;
  label: string;
  unit: string;
  min: number;
  max: number;
  warningThreshold: number;
  criticalThreshold: number;
  decimals: number;
}

interface GaugeSnapshot {
  value: number;
}

const GAUGES: GaugeConfig[] = [
  { id: "rpm",      label: "RPM",            unit: "rpm",   min: 0,  max: 4000, warningThreshold: 3200, criticalThreshold: 3600, decimals: 0 },
  { id: "boost",    label: "Boost",          unit: "bar",   min: 0,  max: 3.5,  warningThreshold: 2.5,  criticalThreshold: 3.0,  decimals: 2 },
  { id: "coolant",  label: "Coolant Temp",   unit: "\u00B0C",  min: 0,  max: 120,  warningThreshold: 95,   criticalThreshold: 105,  decimals: 1 },
  { id: "oil",      label: "Oil Pressure",   unit: "bar",   min: 0,  max: 8,    warningThreshold: 2.0,  criticalThreshold: 1.0,  decimals: 1 },
  { id: "rail",     label: "Rail Pressure",  unit: "bar",   min: 0,  max: 2500, warningThreshold: 2000, criticalThreshold: 2300, decimals: 0 },
  { id: "egt",      label: "EGT",            unit: "\u00B0C",  min: 0,  max: 900,  warningThreshold: 700,  criticalThreshold: 820,  decimals: 0 },
  { id: "battery",  label: "Battery",        unit: "V",     min: 10, max: 15,   warningThreshold: 11.5, criticalThreshold: 11.0, decimals: 1 },
  { id: "speed",    label: "Speed",          unit: "km/h",  min: 0,  max: 120,  warningThreshold: 100,  criticalThreshold: 115,  decimals: 0 },
];

const HISTORY_LENGTH = 20;

function simulateValue(gauge: GaugeConfig, prevValue: number | null): number {
  if (prevValue === null) {
    const range = gauge.max - gauge.min;
    return gauge.min + range * 0.3 + Math.random() * range * 0.3;
  }
  const range = gauge.max - gauge.min;
  const drift = (Math.random() - 0.48) * range * 0.06;
  const next = prevValue + drift;
  return Math.max(gauge.min, Math.min(gauge.max, next));
}

function getColor(value: number, gauge: GaugeConfig): "green" | "yellow" | "red" {
  // Oil pressure is inverted: low is dangerous
  if (gauge.id === "oil" || gauge.id === "battery") {
    if (value <= gauge.criticalThreshold) return "red";
    if (value <= gauge.warningThreshold) return "yellow";
    return "green";
  }
  if (value >= gauge.criticalThreshold) return "red";
  if (value >= gauge.warningThreshold) return "yellow";
  return "green";
}

const COLOR_MAP = {
  green: { ring: "text-green-500", bg: "bg-green-500", text: "text-green-400", line: "#22c55e" },
  yellow: { ring: "text-yellow-500", bg: "bg-yellow-500", text: "text-yellow-400", line: "#eab308" },
  red: { ring: "text-red-500", bg: "bg-red-500", text: "text-red-400", line: "#ef4444" },
};

export function LiveDataPanel() {
  const [recording, setRecording] = useState(false);
  const [running, setRunning] = useState(false);
  const [history, setHistory] = useState<Record<string, GaugeSnapshot[]>>(() => {
    const init: Record<string, GaugeSnapshot[]> = {};
    for (const g of GAUGES) init[g.id] = [];
    return init;
  });
  const [currentValues, setCurrentValues] = useState<Record<string, number>>({});
  const [elapsed, setElapsed] = useState(0);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const tick = useCallback(() => {
    setCurrentValues((prev) => {
      const next: Record<string, number> = {};
      for (const g of GAUGES) {
        next[g.id] = simulateValue(g, prev[g.id] ?? null);
      }
      return next;
    });
  }, []);

  useEffect(() => {
    // Update history when values change
    if (Object.keys(currentValues).length === 0) return;
    setHistory((prev) => {
      const next = { ...prev };
      for (const g of GAUGES) {
        const arr = [...(prev[g.id] || []), { value: currentValues[g.id] ?? 0 }];
        next[g.id] = arr.slice(-HISTORY_LENGTH);
      }
      return next;
    });
  }, [currentValues]);

  const handleStartStop = () => {
    if (running) {
      if (intervalRef.current) clearInterval(intervalRef.current);
      if (timerRef.current) clearInterval(timerRef.current);
      intervalRef.current = null;
      timerRef.current = null;
      setRunning(false);
    } else {
      tick(); // initial tick
      intervalRef.current = setInterval(tick, 500);
      timerRef.current = setInterval(() => setElapsed((e) => e + 1), 1000);
      setRunning(true);
    }
  };

  const handleRecord = () => {
    if (!running && !recording) handleStartStop();
    setRecording(!recording);
  };

  useEffect(() => {
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, []);

  const formatElapsed = (s: number) => {
    const m = Math.floor(s / 60);
    const sec = s % 60;
    return `${m.toString().padStart(2, "0")}:${sec.toString().padStart(2, "0")}`;
  };

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-zinc-800">
        <div className="flex items-center gap-3">
          <h2 className="text-sm font-semibold text-zinc-200">Live Data</h2>
          {running && (
            <span className="flex items-center gap-1.5 text-xs text-green-400">
              <span className="w-1.5 h-1.5 rounded-full bg-green-400 animate-pulse" />
              LIVE
            </span>
          )}
          {running && (
            <span className="text-xs text-zinc-500 font-mono">{formatElapsed(elapsed)}</span>
          )}
        </div>
        <div className="flex gap-2">
          <button
            onClick={handleRecord}
            className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-medium transition-colors ${
              recording
                ? "bg-red-500/20 text-red-400 border border-red-500/30"
                : "bg-zinc-800 text-zinc-400 border border-zinc-700 hover:text-zinc-200"
            }`}
          >
            <Circle size={12} className={recording ? "fill-red-400" : ""} />
            {recording ? "Recording" : "Record"}
          </button>
          <button
            onClick={handleStartStop}
            className={`flex items-center gap-2 px-4 py-1.5 rounded-lg text-xs font-medium transition-colors ${
              running
                ? "bg-red-500/20 text-red-400 border border-red-500/30"
                : "bg-green-500/20 text-green-400 border border-green-500/30"
            }`}
          >
            {running ? <Square size={12} /> : <Play size={12} />}
            {running ? "Stop" : "Start"}
          </button>
        </div>
      </div>

      {/* Gauge grid */}
      <div className="flex-1 overflow-auto p-4">
        <div className="grid grid-cols-4 gap-4">
          {GAUGES.map((gauge) => {
            const value = currentValues[gauge.id];
            const hasValue = value !== undefined;
            const color = hasValue ? getColor(value, gauge) : "green";
            const colors = COLOR_MAP[color];
            const pct = hasValue
              ? ((value - gauge.min) / (gauge.max - gauge.min)) * 100
              : 0;
            const sparkData = history[gauge.id] || [];

            return (
              <div
                key={gauge.id}
                className="bg-zinc-900/60 border border-zinc-800 rounded-xl p-4 flex flex-col items-center gap-2"
              >
                {/* Circular gauge */}
                <div className="relative w-24 h-24">
                  <svg viewBox="0 0 100 100" className="w-full h-full -rotate-90">
                    {/* Background ring */}
                    <circle
                      cx="50" cy="50" r="42"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="6"
                      className="text-zinc-800"
                    />
                    {/* Value ring */}
                    <circle
                      cx="50" cy="50" r="42"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="6"
                      strokeLinecap="round"
                      strokeDasharray={`${pct * 2.64} 264`}
                      className={`${colors.ring} transition-all duration-300`}
                    />
                  </svg>
                  {/* Center value */}
                  <div className="absolute inset-0 flex flex-col items-center justify-center">
                    <span className={`text-lg font-bold ${hasValue ? colors.text : "text-zinc-600"}`}>
                      {hasValue ? value.toFixed(gauge.decimals) : "--"}
                    </span>
                    <span className="text-[10px] text-zinc-500">{gauge.unit}</span>
                  </div>
                </div>

                {/* Label */}
                <span className="text-xs text-zinc-400 font-medium">{gauge.label}</span>

                {/* Sparkline */}
                <div className="w-full h-8">
                  {sparkData.length > 1 ? (
                    <ResponsiveContainer width="100%" height="100%">
                      <LineChart data={sparkData}>
                        <YAxis domain={[gauge.min, gauge.max]} hide />
                        <Line
                          type="monotone"
                          dataKey="value"
                          stroke={colors.line}
                          strokeWidth={1.5}
                          dot={false}
                          isAnimationActive={false}
                        />
                      </LineChart>
                    </ResponsiveContainer>
                  ) : (
                    <div className="w-full h-full flex items-center justify-center">
                      <span className="text-[10px] text-zinc-700">no data</span>
                    </div>
                  )}
                </div>

                {/* Range */}
                <div className="flex justify-between w-full text-[10px] text-zinc-600">
                  <span>{gauge.min}</span>
                  <span>{gauge.max}</span>
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
