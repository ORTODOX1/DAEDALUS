import React, { useState, useMemo, useCallback, useRef } from 'react';
import { GitCompareArrows, ChevronDown, ChevronUp, ArrowRight, FileWarning } from 'lucide-react';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------
interface DiffRegion {
  address: number;
  length: number;
  label: string;
  description: string;
  stockBytes: number[];
  modifiedBytes: number[];
}

// ---------------------------------------------------------------------------
// Mock diff regions — typical tuning changes
// ---------------------------------------------------------------------------
const MOCK_REGIONS: DiffRegion[] = [
  {
    address: 0x1a000,
    length: 12,
    label: 'Boost pressure map (row 3)',
    description: 'Increased boost by ~150 mbar at mid-RPM',
    stockBytes:    [0x08, 0x98, 0x09, 0x60, 0x0a, 0x28, 0x0a, 0xf0, 0x0b, 0xb8, 0x0c, 0x1c],
    modifiedBytes: [0x09, 0x60, 0x0a, 0x28, 0x0b, 0x54, 0x0b, 0xb8, 0x0c, 0x80, 0x0c, 0xe4],
  },
  {
    address: 0x1c200,
    length: 8,
    label: 'Injection timing (pilot)',
    description: 'Advanced pilot injection by 2 degrees',
    stockBytes:    [0x1e, 0x22, 0x26, 0x2a, 0x2e, 0x32, 0x36, 0x3a],
    modifiedBytes: [0x22, 0x26, 0x2a, 0x2e, 0x32, 0x36, 0x3a, 0x3e],
  },
  {
    address: 0x24800,
    length: 6,
    label: 'Rail pressure limiter',
    description: 'Raised max rail pressure from 1800 to 2000 bar',
    stockBytes:    [0x07, 0x08, 0x07, 0x08, 0x07, 0x08],
    modifiedBytes: [0x07, 0xd0, 0x07, 0xd0, 0x07, 0xd0],
  },
  {
    address: 0x31400,
    length: 10,
    label: 'Torque limiter table',
    description: 'Increased torque limit +40 Nm above 2000 RPM',
    stockBytes:    [0x01, 0x2c, 0x01, 0x5e, 0x01, 0x90, 0x01, 0xc2, 0x01, 0xf4],
    modifiedBytes: [0x01, 0x2c, 0x01, 0x86, 0x01, 0xc2, 0x01, 0xfe, 0x02, 0x26],
  },
  {
    address: 0x38c00,
    length: 4,
    label: 'Speed limiter',
    description: 'Removed electronic speed limiter (set to max)',
    stockBytes:    [0x00, 0xfa, 0x00, 0xfa],
    modifiedBytes: [0xff, 0xff, 0xff, 0xff],
  },
];

const BYTES_PER_LINE = 16;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function toHex(n: number, pad: number): string {
  return n.toString(16).toUpperCase().padStart(pad, '0');
}

function toAscii(b: number): string {
  return b >= 0x20 && b <= 0x7e ? String.fromCharCode(b) : '.';
}

/** Pad region bytes to full lines for display */
function padToLines(region: DiffRegion): {
  lines: number;
  startAddr: number;
} {
  const lineStart = Math.floor(region.address / BYTES_PER_LINE) * BYTES_PER_LINE;
  const endAddr = region.address + region.length;
  const lineEnd = Math.ceil(endAddr / BYTES_PER_LINE) * BYTES_PER_LINE;
  return { lines: (lineEnd - lineStart) / BYTES_PER_LINE, startAddr: lineStart };
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------
function RegionPanel({
  region,
  expanded,
  onToggle,
}: {
  region: DiffRegion;
  expanded: boolean;
  onToggle: () => void;
}) {
  const { lines, startAddr } = padToLines(region);
  const diffStart = region.address;
  const diffEnd = region.address + region.length;

  const renderSide = (bytes: number[], side: 'stock' | 'modified') => {
    const result: React.ReactNode[] = [];

    for (let line = 0; line < lines; line++) {
      const lineAddr = startAddr + line * BYTES_PER_LINE;
      const hexParts: React.ReactNode[] = [];
      const asciiParts: React.ReactNode[] = [];

      for (let col = 0; col < BYTES_PER_LINE; col++) {
        const addr = lineAddr + col;
        const inRegion = addr >= diffStart && addr < diffEnd;
        const byteIdx = addr - diffStart;

        if (inRegion && byteIdx >= 0 && byteIdx < bytes.length) {
          const val = bytes[byteIdx];
          const otherVal = side === 'stock' ? region.modifiedBytes[byteIdx] : region.stockBytes[byteIdx];
          const changed = val !== otherVal;

          hexParts.push(
            <span
              key={col}
              className={`px-[1px] rounded-sm ${
                changed
                  ? side === 'stock'
                    ? 'bg-red-500/25 text-red-300'
                    : 'bg-green-500/25 text-green-300'
                  : 'text-zinc-400'
              }`}
            >
              {toHex(val, 2)}
            </span>,
          );
          asciiParts.push(
            <span
              key={col}
              className={changed ? (side === 'stock' ? 'text-red-300' : 'text-green-300') : 'text-zinc-500'}
            >
              {toAscii(val)}
            </span>,
          );
        } else {
          hexParts.push(
            <span key={col} className="text-zinc-600 px-[1px]">
              {'..'}
            </span>,
          );
          asciiParts.push(
            <span key={col} className="text-zinc-700">
              .
            </span>,
          );
        }
        if (col === 7) hexParts.push(<span key="sp" className="w-1.5 inline-block" />);
      }

      result.push(
        <div key={line} className="flex items-center h-5 leading-5 whitespace-nowrap font-mono text-[12px]">
          <span className="text-amber-400/70 w-[72px] shrink-0 select-none">{toHex(lineAddr, 8)}</span>
          <span className="flex gap-[2px] mr-3">{hexParts}</span>
          <span className="tracking-[1px]">{asciiParts}</span>
        </div>,
      );
    }

    return result;
  };

  return (
    <div className="border border-zinc-700 rounded-lg overflow-hidden">
      {/* Region header */}
      <button
        className="w-full flex items-center gap-2 px-3 py-2 bg-zinc-800 hover:bg-zinc-750 text-left transition-colors"
        onClick={onToggle}
      >
        {expanded ? <ChevronUp size={14} className="text-zinc-400" /> : <ChevronDown size={14} className="text-zinc-400" />}
        <span className="text-amber-400 font-mono text-xs">0x{toHex(region.address, 8)}</span>
        <span className="text-zinc-100 text-sm font-medium">{region.label}</span>
        <span className="text-zinc-500 text-xs ml-auto">{region.length} bytes</span>
      </button>

      {expanded && (
        <>
          <div className="px-3 py-1.5 bg-zinc-800/50 border-t border-zinc-700 text-xs text-zinc-400">
            {region.description}
          </div>
          <div className="flex border-t border-zinc-700">
            {/* Stock side */}
            <div className="flex-1 border-r border-zinc-700">
              <div className="px-3 py-1 bg-red-500/10 border-b border-zinc-700 text-[11px] text-red-400 font-semibold select-none">
                STOCK
              </div>
              <div className="px-3 py-1.5 bg-zinc-900">{renderSide(region.stockBytes, 'stock')}</div>
            </div>
            {/* Modified side */}
            <div className="flex-1">
              <div className="px-3 py-1 bg-green-500/10 border-b border-zinc-700 text-[11px] text-green-400 font-semibold select-none">
                MODIFIED
              </div>
              <div className="px-3 py-1.5 bg-zinc-900">{renderSide(region.modifiedBytes, 'modified')}</div>
            </div>
          </div>
        </>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main Component
// ---------------------------------------------------------------------------
export default function DiffView() {
  const [expandedRegions, setExpandedRegions] = useState<Set<number>>(() => new Set([0, 4]));

  const toggleRegion = useCallback((idx: number) => {
    setExpandedRegions((prev) => {
      const next = new Set(prev);
      if (next.has(idx)) next.delete(idx);
      else next.add(idx);
      return next;
    });
  }, []);

  const expandAll = useCallback(() => {
    setExpandedRegions(new Set(MOCK_REGIONS.map((_, i) => i)));
  }, []);

  const collapseAll = useCallback(() => {
    setExpandedRegions(new Set());
  }, []);

  // Summary stats
  const totalBytesChanged = useMemo(
    () =>
      MOCK_REGIONS.reduce((acc, r) => {
        let changed = 0;
        for (let i = 0; i < r.length; i++) {
          if (r.stockBytes[i] !== r.modifiedBytes[i]) changed++;
        }
        return acc + changed;
      }, 0),
    [],
  );

  const scrollRefs = useRef<(HTMLDivElement | null)[]>([]);

  const jumpToRegion = useCallback(
    (idx: number) => {
      setExpandedRegions((prev) => new Set([...prev, idx]));
      setTimeout(() => {
        scrollRefs.current[idx]?.scrollIntoView({ behavior: 'smooth', block: 'center' });
      }, 50);
    },
    [],
  );

  return (
    <div className="flex flex-col h-full bg-zinc-900 text-zinc-100 text-sm rounded-lg border border-zinc-700 overflow-hidden">
      {/* ---- Header ---- */}
      <div className="flex items-center gap-3 px-3 py-2 bg-zinc-800 border-b border-zinc-700 flex-wrap">
        <GitCompareArrows size={16} className="text-sky-400" />
        <span className="font-semibold">Diff View</span>
        <span className="text-zinc-500">|</span>
        <span className="text-zinc-400 text-xs">stock_EDC17C46.bin</span>
        <ArrowRight size={12} className="text-zinc-500" />
        <span className="text-zinc-400 text-xs">modified_EDC17C46.bin</span>

        <div className="ml-auto flex items-center gap-2">
          <button className="px-2 py-1 text-xs bg-zinc-700 hover:bg-zinc-600 rounded" onClick={expandAll}>
            Expand all
          </button>
          <button className="px-2 py-1 text-xs bg-zinc-700 hover:bg-zinc-600 rounded" onClick={collapseAll}>
            Collapse all
          </button>
        </div>
      </div>

      {/* ---- Summary bar ---- */}
      <div className="flex items-center gap-4 px-3 py-2 bg-zinc-800/50 border-b border-zinc-700 text-xs">
        <div className="flex items-center gap-1.5">
          <FileWarning size={13} className="text-amber-400" />
          <span className="text-zinc-300">
            <span className="text-amber-400 font-semibold">{MOCK_REGIONS.length}</span> regions changed
          </span>
        </div>
        <span className="text-zinc-600">|</span>
        <span className="text-zinc-300">
          <span className="text-sky-400 font-semibold">{totalBytesChanged}</span> bytes modified
        </span>
        <span className="text-zinc-600">|</span>

        {/* Jump-to buttons */}
        <div className="flex items-center gap-1 flex-wrap">
          <span className="text-zinc-500 mr-1">Jump to:</span>
          {MOCK_REGIONS.map((r, i) => (
            <button
              key={i}
              className="px-1.5 py-0.5 rounded bg-zinc-700 hover:bg-zinc-600 text-zinc-300 text-[11px] transition-colors"
              onClick={() => jumpToRegion(i)}
            >
              {r.label.length > 24 ? r.label.slice(0, 22) + '...' : r.label}
            </button>
          ))}
        </div>
      </div>

      {/* ---- Diff regions ---- */}
      <div className="flex-1 overflow-y-auto p-3 space-y-3">
        {MOCK_REGIONS.map((region, idx) => (
          <div key={idx} ref={(el) => { scrollRefs.current[idx] = el; }}>
            <RegionPanel
              region={region}
              expanded={expandedRegions.has(idx)}
              onToggle={() => toggleRegion(idx)}
            />
          </div>
        ))}

        {MOCK_REGIONS.length === 0 && (
          <div className="flex flex-col items-center justify-center h-full text-zinc-500 gap-2">
            <GitCompareArrows size={32} />
            <span>No differences found</span>
          </div>
        )}
      </div>

      {/* ---- Footer ---- */}
      <div className="flex items-center justify-between px-3 py-1.5 bg-zinc-800 border-t border-zinc-700 text-xs text-zinc-400">
        <span>
          Comparing <span className="text-zinc-300">stock_EDC17C46.bin</span> (524,288 bytes)
          {' vs '}
          <span className="text-zinc-300">modified_EDC17C46.bin</span> (524,288 bytes)
        </span>
        <span className="flex items-center gap-3">
          <span className="flex items-center gap-1">
            <span className="inline-block w-2 h-2 rounded-sm bg-red-500/40" /> Removed
          </span>
          <span className="flex items-center gap-1">
            <span className="inline-block w-2 h-2 rounded-sm bg-green-500/40" /> Added
          </span>
        </span>
      </div>
    </div>
  );
}
