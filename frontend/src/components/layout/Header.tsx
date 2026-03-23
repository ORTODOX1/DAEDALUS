import { Menu, Maximize2, Minus, X } from "lucide-react";
import { useUIStore } from "../../stores/uiStore";
import { useConnectionStore } from "../../stores/connectionStore";

export function Header() {
  const toggleSidebar = useUIStore((s) => s.toggleSidebar);
  const activePanel = useUIStore((s) => s.activePanel);
  const ecuInfo = useConnectionStore((s) => s.ecuInfo);

  const panelTitles: Record<string, string> = {
    connection: "Connection Manager",
    dtc: "DTC Diagnostic Codes",
    live: "Live Data Monitor",
    flash: "Flash Read/Write",
    editor: "Map Editor",
    ai: "AI Assistant",
    reverse: "Reverse Engineering",
    settings: "Settings",
  };

  return (
    <header className="h-12 bg-zinc-900 border-b border-zinc-800 flex items-center px-4 gap-4">
      <button
        onClick={toggleSidebar}
        className="text-zinc-400 hover:text-zinc-200 transition-colors"
      >
        <Menu size={18} />
      </button>

      <h2 className="text-sm font-medium text-zinc-200">
        {panelTitles[activePanel] ?? activePanel}
      </h2>

      {ecuInfo && (
        <div className="ml-4 flex items-center gap-2 text-xs text-zinc-500">
          <span className="px-2 py-0.5 bg-zinc-800 rounded">{ecuInfo.manufacturer}</span>
          <span className="px-2 py-0.5 bg-zinc-800 rounded">{ecuInfo.name}</span>
          <span className="px-2 py-0.5 bg-zinc-800 rounded">{ecuInfo.processor}</span>
        </div>
      )}

      <div className="flex-1" />

      <span className="text-xs text-zinc-600">v0.1.0</span>
    </header>
  );
}
