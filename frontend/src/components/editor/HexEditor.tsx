import React, { useState, useCallback, useMemo, useRef, useEffect } from 'react';
import { Search, ArrowRight, ChevronUp, ChevronDown } from 'lucide-react';

// ---------------------------------------------------------------------------
// Mock data generator — 64 KB of semi-random ECU firmware
// ---------------------------------------------------------------------------
function generateMockFirmware(size: number): Uint8Array {
  const data = new Uint8Array(size);

  // Header — "MED17.5.21\0"
  const header = [0x4d, 0x45, 0x44, 0x31, 0x37, 0x2e, 0x35, 0x2e, 0x32, 0x31, 0x00];
  data.set(header, 0);

  // Fill body with pseudo-random bytes seeded by offset
  for (let i = header.length; i < size; i++) {
    data[i] = ((i * 7 + 0x5a) ^ ((i >> 8) * 13)) & 0xff;
  }

  // Inject a recognisable calibration block at 0x1A00
  for (let i = 0; i < 128; i++) {
    data[0x1a00 + i] = Math.min(255, Math.round(100 + 120 * Math.sin(i / 10)));
  }

  // Inject ASCII string at 0x3000
  const id = 'BOSCH_EDC17C46_VW_2.0TDI';
  for (let i = 0; i < id.length; i++) data[0x3000 + i] = id.charCodeAt(i);

  return data;
}

// ---------------------------------------------------------------------------
// Highlighted regions (e.g. maps found by AI)
// ---------------------------------------------------------------------------
interface HighlightRegion {
  start: number;
  end: number;
  label: string;
  color: 'blue' | 'amber' | 'green' | 'purple';
}

const MOCK_REGIONS: HighlightRegion[] = [
  { start: 0x0000, end: 0x000b, label: 'Header', color: 'purple' },
  { start: 0x1a00, end: 0x1a7f, label: 'Boost map', color: 'blue' },
  { start: 0x3000, end: 0x3018, label: 'ECU ID string', color: 'amber' },
];

const REGION_BG: Record<HighlightRegion['color'], string> = {
  blue: 'bg-blue-500/10',
  amber: 'bg-amber-500/10',
  green: 'bg-green-500/10',
  purple: 'bg-purple-500/10',
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
const BYTES_PER_ROW = 16;
const VISIBLE_ROWS = 32;
const ROW_HEIGHT = 24; // px

function toHex(n: number, pad: number): string {
  return n.toString(16).toUpperCase().padStart(pad, '0');
}

function toAscii(b: number): string {
  return b >= 0x20 && b <= 0x7e ? String.fromCharCode(b) : '.';
}

function regionForByte(offset: number, regions: HighlightRegion[]): HighlightRegion | undefined {
  return regions.find((r) => offset >= r.start && offset <= r.end);
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export default function HexEditor() {
  const firmware = useMemo(() => generateMockFirmware(65536), []);
  const totalRows = Math.ceil(firmware.length / BYTES_PER_ROW);

  const [scrollTop, setScrollTop] = useState(0);
  const [selectedByte, setSelectedByte] = useState<number | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [gotoAddr, setGotoAddr] = useState('');
  const [searchResults, setSearchResults] = useState<number[]>([]);
  const [currentResult, setCurrentResult] = useState(-1);

  const containerRef = useRef<HTMLDivElement>(null);

  // Derived visible window
  const firstRow = Math.floor(scrollTop / ROW_HEIGHT);
  const visibleRows = Math.min(VISIBLE_ROWS, totalRows - firstRow);

  // ---------- scroll handler ----------
  const onScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
    setScrollTop(e.currentTarget.scrollTop);
  }, []);

  // ---------- go-to address ----------
  const handleGoto = useCallback(() => {
    const addr = parseInt(gotoAddr, 16);
    if (isNaN(addr) || addr < 0 || addr >= firmware.length) return;
    const row = Math.floor(addr / BYTES_PER_ROW);
    const newTop = row * ROW_HEIGHT;
    if (containerRef.current) containerRef.current.scrollTop = newTop;
    setSelectedByte(addr);
  }, [gotoAddr, firmware.length]);

  // ---------- search (hex or ASCII) ----------
  const handleSearch = useCallback(() => {
    if (!searchQuery.trim()) {
      setSearchResults([]);
      setCurrentResult(-1);
      return;
    }

    let needle: number[] = [];

    // Try parsing as hex sequence first (e.g. "4D 45 44" or "4D4544")
    const hexCleaned = searchQuery.replace(/\s+/g, '');
    if (/^[0-9a-fA-F]+$/.test(hexCleaned) && hexCleaned.length % 2 === 0) {
      for (let i = 0; i < hexCleaned.length; i += 2) {
        needle.push(parseInt(hexCleaned.substring(i, i + 2), 16));
      }
    } else {
      // Treat as ASCII
      needle = Array.from(searchQuery).map((c) => c.charCodeAt(0));
    }

    if (needle.length === 0) return;

    const results: number[] = [];
    for (let i = 0; i <= firmware.length - needle.length; i++) {
      let match = true;
      for (let j = 0; j < needle.length; j++) {
        if (firmware[i + j] !== needle[j]) { match = false; break; }
      }
      if (match) results.push(i);
    }

    setSearchResults(results);
    if (results.length > 0) {
      setCurrentResult(0);
      jumpTo(results[0]);
    } else {
      setCurrentResult(-1);
    }
  }, [searchQuery, firmware]);

  const jumpTo = useCallback((addr: number) => {
    const row = Math.floor(addr / BYTES_PER_ROW);
    const newTop = Math.max(0, (row - 4) * ROW_HEIGHT);
    if (containerRef.current) containerRef.current.scrollTop = newTop;
    setSelectedByte(addr);
  }, []);

  const navigateResult = useCallback(
    (dir: 1 | -1) => {
      if (searchResults.length === 0) return;
      const next = (currentResult + dir + searchResults.length) % searchResults.length;
      setCurrentResult(next);
      jumpTo(searchResults[next]);
    },
    [searchResults, currentResult, jumpTo],
  );

  // ---------- keyboard nav ----------
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Enter' && e.target instanceof HTMLInputElement) return;
      if (selectedByte === null) return;
      let next = selectedByte;
      if (e.key === 'ArrowRight') next = Math.min(firmware.length - 1, selectedByte + 1);
      if (e.key === 'ArrowLeft') next = Math.max(0, selectedByte - 1);
      if (e.key === 'ArrowDown') next = Math.min(firmware.length - 1, selectedByte + BYTES_PER_ROW);
      if (e.key === 'ArrowUp') next = Math.max(0, selectedByte - BYTES_PER_ROW);
      if (next !== selectedByte) {
        e.preventDefault();
        setSelectedByte(next);
        // auto-scroll if needed
        const row = Math.floor(next / BYTES_PER_ROW);
        if (containerRef.current) {
          const st = containerRef.current.scrollTop;
          const topRow = Math.floor(st / ROW_HEIGHT);
          if (row < topRow) containerRef.current.scrollTop = row * ROW_HEIGHT;
          if (row >= topRow + VISIBLE_ROWS - 1)
            containerRef.current.scrollTop = (row - VISIBLE_ROWS + 2) * ROW_HEIGHT;
        }
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [selectedByte, firmware.length]);

  // ---------- render rows ----------
  const rows = useMemo(() => {
    const out: React.ReactNode[] = [];
    for (let r = 0; r < visibleRows; r++) {
      const rowIdx = firstRow + r;
      const baseAddr = rowIdx * BYTES_PER_ROW;
      if (baseAddr >= firmware.length) break;

      const hexCells: React.ReactNode[] = [];
      const asciiCells: React.ReactNode[] = [];

      for (let col = 0; col < BYTES_PER_ROW; col++) {
        const addr = baseAddr + col;
        if (addr >= firmware.length) break;
        const val = firmware[addr];
        const region = regionForByte(addr, MOCK_REGIONS);
        const isSelected = addr === selectedByte;
        const isSearchHit = searchResults.includes(addr);

        let cellBg = '';
        if (isSelected) cellBg = 'bg-sky-600/60';
        else if (isSearchHit) cellBg = 'bg-yellow-500/30';
        else if (region) cellBg = REGION_BG[region.color];

        hexCells.push(
          <span
            key={`h${col}`}
            className={`cursor-pointer px-[2px] rounded-sm hover:bg-zinc-700 ${cellBg}`}
            onClick={() => setSelectedByte(addr)}
            title={region?.label}
          >
            {toHex(val, 2)}
          </span>,
        );

        asciiCells.push(
          <span
            key={`a${col}`}
            className={`cursor-pointer hover:bg-zinc-700 ${cellBg}`}
            onClick={() => setSelectedByte(addr)}
          >
            {toAscii(val)}
          </span>,
        );

        // Insert extra space after byte 7 for readability
        if (col === 7) hexCells.push(<span key="sp" className="w-2 inline-block" />);
      }

      out.push(
        <div key={rowIdx} className="flex items-center h-6 leading-6 whitespace-nowrap">
          <span className="text-amber-400 w-[80px] shrink-0 select-none">{toHex(baseAddr, 8)}</span>
          <span className="text-zinc-300 tracking-wider mr-4 select-text flex gap-[3px]">{hexCells}</span>
          <span className="text-green-400 select-text tracking-[1px]">{asciiCells}</span>
        </div>,
      );
    }
    return out;
  }, [firstRow, visibleRows, firmware, selectedByte, searchResults]);

  // ---------- selected byte info ----------
  const selInfo = useMemo(() => {
    if (selectedByte === null) return null;
    const val = firmware[selectedByte];
    const region = regionForByte(selectedByte, MOCK_REGIONS);
    return { addr: selectedByte, val, ascii: toAscii(val), region };
  }, [selectedByte, firmware]);

  return (
    <div className="flex flex-col h-full bg-zinc-900 text-zinc-100 font-mono text-sm rounded-lg border border-zinc-700 overflow-hidden">
      {/* ---- Toolbar ---- */}
      <div className="flex items-center gap-3 px-3 py-2 bg-zinc-800 border-b border-zinc-700 flex-wrap">
        {/* Search */}
        <div className="flex items-center gap-1 bg-zinc-900 rounded px-2 py-1 border border-zinc-600 focus-within:border-sky-500">
          <Search size={14} className="text-zinc-400" />
          <input
            className="bg-transparent outline-none text-zinc-100 w-40 placeholder:text-zinc-500"
            placeholder="Hex or ASCII..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
          />
        </div>
        <button
          className="px-2 py-1 bg-sky-600 hover:bg-sky-500 rounded text-xs font-semibold"
          onClick={handleSearch}
        >
          Search
        </button>
        {searchResults.length > 0 && (
          <div className="flex items-center gap-1 text-xs text-zinc-400">
            <button onClick={() => navigateResult(-1)} className="hover:text-zinc-100"><ChevronUp size={14} /></button>
            <span>{currentResult + 1}/{searchResults.length}</span>
            <button onClick={() => navigateResult(1)} className="hover:text-zinc-100"><ChevronDown size={14} /></button>
          </div>
        )}

        <div className="w-px h-5 bg-zinc-600" />

        {/* Go to address */}
        <div className="flex items-center gap-1 bg-zinc-900 rounded px-2 py-1 border border-zinc-600 focus-within:border-sky-500">
          <span className="text-zinc-500 text-xs">0x</span>
          <input
            className="bg-transparent outline-none text-zinc-100 w-20 placeholder:text-zinc-500"
            placeholder="Address"
            value={gotoAddr}
            onChange={(e) => setGotoAddr(e.target.value.replace(/[^0-9a-fA-F]/g, ''))}
            onKeyDown={(e) => e.key === 'Enter' && handleGoto()}
          />
        </div>
        <button
          className="p-1 bg-zinc-700 hover:bg-zinc-600 rounded"
          onClick={handleGoto}
          title="Go to address"
        >
          <ArrowRight size={14} />
        </button>

        {/* Regions legend */}
        <div className="ml-auto flex items-center gap-3 text-[11px]">
          {MOCK_REGIONS.map((r) => (
            <span
              key={r.label}
              className={`px-1.5 py-0.5 rounded ${REGION_BG[r.color]} border border-zinc-600`}
            >
              {r.label}
            </span>
          ))}
        </div>
      </div>

      {/* ---- Column header ---- */}
      <div className="flex items-center h-6 px-3 bg-zinc-800/60 border-b border-zinc-700 text-zinc-500 text-[11px] select-none">
        <span className="w-[80px] shrink-0">Address</span>
        <span className="tracking-wider mr-4 flex gap-[3px]">
          {Array.from({ length: 16 }, (_, i) => (
            <span key={i} className="inline-block w-[18px] text-center">
              {toHex(i, 2)}
            </span>
          ))}
        </span>
        <span className="tracking-[1px]">ASCII</span>
      </div>

      {/* ---- Virtual-scroll hex body ---- */}
      <div
        ref={containerRef}
        className="flex-1 overflow-y-auto px-3"
        onScroll={onScroll}
        tabIndex={0}
      >
        {/* spacer top */}
        <div style={{ height: firstRow * ROW_HEIGHT }} />
        {rows}
        {/* spacer bottom */}
        <div style={{ height: Math.max(0, (totalRows - firstRow - visibleRows) * ROW_HEIGHT) }} />
      </div>

      {/* ---- Status bar ---- */}
      <div className="flex items-center justify-between px-3 py-1.5 bg-zinc-800 border-t border-zinc-700 text-xs text-zinc-400">
        {selInfo ? (
          <span>
            Selected:{' '}
            <span className="text-amber-400">0x{toHex(selInfo.addr, 8)}</span>
            {' = '}
            <span className="text-sky-400">0x{toHex(selInfo.val, 2)}</span>
            {' ('}
            {selInfo.val}
            {') '}
            <span className="text-green-400">'{selInfo.ascii}'</span>
            {selInfo.region && (
              <span className="ml-2 text-zinc-500">| Region: {selInfo.region.label}</span>
            )}
          </span>
        ) : (
          <span>Click a byte to inspect</span>
        )}
        <span>
          Size: {(firmware.length / 1024).toFixed(0)} KB ({firmware.length.toLocaleString()} bytes)
        </span>
      </div>
    </div>
  );
}
