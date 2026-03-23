import { useState } from "react";
import { Search, Trash2, Download, Filter } from "lucide-react";
import type { J1939DTC } from "../../types";

const MOCK_DTCS: J1939DTC[] = [
  { spn: 157, fmi: 0, name: "Fuel Rail Pressure", description: "Common rail fuel pressure above normal (ТНВД overpressure)", category: "fuel_system", severity: "critical", ecu: ["EDC17", "MD1"] },
  { spn: 157, fmi: 1, name: "Fuel Rail Pressure", description: "Common rail fuel pressure below normal (ТНВД underpressure / leak)", category: "fuel_system", severity: "critical", ecu: ["EDC17", "MD1"] },
  { spn: 102, fmi: 0, name: "Boost Pressure", description: "Boost pressure above normal (turbo overboost)", category: "air_system", severity: "critical", ecu: ["EDC17", "CM2350"] },
  { spn: 3226, fmi: 0, name: "DPF Differential Pressure", description: "DPF soot load above threshold — regeneration required", category: "aftertreatment", severity: "warning", ecu: ["EDC17", "MD1"] },
  { spn: 520192, fmi: 0, name: "SCR System (AdBlue)", description: "SCR catalyst efficiency below threshold — DEF quality issue", category: "aftertreatment", severity: "warning", ecu: ["EDC17", "MD1"] },
  { spn: 3216, fmi: 7, name: "EGR Valve Position", description: "EGR valve mechanical failure — not responding", category: "emissions", severity: "critical", ecu: ["EDC17", "MD1"] },
  { spn: 91, fmi: 3, name: "Accelerator Pedal Position", description: "Accelerator pedal position sensor voltage above normal", category: "engine_control", severity: "warning", ecu: ["EDC17", "CM2350"] },
  { spn: 639, fmi: 2, name: "J1939 CAN Bus", description: "J1939 data link erratic — CAN communication error", category: "communication", severity: "warning", ecu: ["EDC17", "CM2350"] },
];

const CATEGORIES = ["all", "fuel_system", "air_system", "aftertreatment", "emissions", "engine_control", "communication"];

const severityColors = {
  critical: "text-red-400 bg-red-500/10 border-red-500/20",
  warning: "text-yellow-400 bg-yellow-500/10 border-yellow-500/20",
  info: "text-blue-400 bg-blue-500/10 border-blue-500/20",
};

export function DTCViewer() {
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState("all");

  const filtered = MOCK_DTCS.filter((dtc) => {
    const matchSearch = search === "" ||
      dtc.name.toLowerCase().includes(search.toLowerCase()) ||
      dtc.description.toLowerCase().includes(search.toLowerCase()) ||
      `${dtc.spn}`.includes(search);
    const matchCategory = category === "all" || dtc.category === category;
    return matchSearch && matchCategory;
  });

  return (
    <div className="p-6 h-full flex flex-col">
      <div className="flex items-center gap-4 mb-4">
        <h3 className="text-lg font-semibold text-zinc-200">J1939 DTC Codes</h3>
        <span className="text-xs text-zinc-500 bg-zinc-800 px-2 py-0.5 rounded">{filtered.length} codes</span>
        <div className="flex-1" />
        <button className="flex items-center gap-1.5 px-3 py-1.5 text-xs bg-red-500/20 text-red-400 rounded hover:bg-red-500/30 transition-colors">
          <Trash2 size={12} /> Clear All
        </button>
        <button className="flex items-center gap-1.5 px-3 py-1.5 text-xs bg-zinc-800 text-zinc-300 rounded hover:bg-zinc-700 transition-colors">
          <Download size={12} /> Export
        </button>
      </div>

      {/* Search + Filter */}
      <div className="flex gap-3 mb-4">
        <div className="flex-1 relative">
          <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-zinc-500" />
          <input
            type="text"
            placeholder="Search by SPN, name, or description..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="w-full pl-9 pr-3 py-2 bg-zinc-900 border border-zinc-800 rounded text-sm text-zinc-200 placeholder:text-zinc-600 focus:outline-none focus:border-zinc-600"
          />
        </div>
        <div className="flex items-center gap-1.5">
          <Filter size={14} className="text-zinc-500" />
          <select
            value={category}
            onChange={(e) => setCategory(e.target.value)}
            className="bg-zinc-900 border border-zinc-800 rounded text-sm text-zinc-300 px-2 py-2 focus:outline-none"
          >
            {CATEGORIES.map((c) => (
              <option key={c} value={c}>{c === "all" ? "All Categories" : c.replace(/_/g, " ")}</option>
            ))}
          </select>
        </div>
      </div>

      {/* Table */}
      <div className="flex-1 overflow-auto">
        <table className="w-full text-sm">
          <thead className="sticky top-0 bg-zinc-950">
            <tr className="text-left text-xs text-zinc-500 border-b border-zinc-800">
              <th className="py-2 px-3 w-20">SPN</th>
              <th className="py-2 px-3 w-12">FMI</th>
              <th className="py-2 px-3">Name</th>
              <th className="py-2 px-3">Description</th>
              <th className="py-2 px-3 w-24">Severity</th>
              <th className="py-2 px-3 w-32">ECU</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((dtc, i) => (
              <tr key={`${dtc.spn}-${dtc.fmi}-${i}`} className="border-b border-zinc-800/50 hover:bg-zinc-900/50">
                <td className="py-2.5 px-3 font-mono text-amber-400">{dtc.spn}</td>
                <td className="py-2.5 px-3 font-mono text-zinc-400">{dtc.fmi}</td>
                <td className="py-2.5 px-3 text-zinc-200">{dtc.name}</td>
                <td className="py-2.5 px-3 text-zinc-400">{dtc.description}</td>
                <td className="py-2.5 px-3">
                  <span className={`text-xs px-2 py-0.5 rounded border ${severityColors[dtc.severity]}`}>
                    {dtc.severity}
                  </span>
                </td>
                <td className="py-2.5 px-3 text-xs text-zinc-500">{dtc.ecu.join(", ")}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
