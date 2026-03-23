import {
  Plug,
  AlertTriangle,
  Activity,
  Cpu,
  FileCode,
  Bot,
  Search,
  Settings,
} from "lucide-react";
import { useUIStore } from "../../stores/uiStore";
import { useConnectionStore } from "../../stores/connectionStore";
import type { Panel } from "../../types";

const navItems: { id: Panel; label: string; icon: React.ReactNode }[] = [
  { id: "connection", label: "Connection", icon: <Plug size={20} /> },
  { id: "dtc", label: "DTC Codes", icon: <AlertTriangle size={20} /> },
  { id: "live", label: "Live Data", icon: <Activity size={20} /> },
  { id: "flash", label: "Flash", icon: <Cpu size={20} /> },
  { id: "editor", label: "Editor", icon: <FileCode size={20} /> },
  { id: "ai", label: "AI Assistant", icon: <Bot size={20} /> },
  { id: "reverse", label: "Reverse Eng", icon: <Search size={20} /> },
];

export function Sidebar() {
  const { activePanel, setActivePanel, sidebarOpen } = useUIStore();
  const status = useConnectionStore((s) => s.status);

  if (!sidebarOpen) return null;

  return (
    <aside className="w-56 bg-zinc-900 border-r border-zinc-800 flex flex-col">
      <div className="p-4 border-b border-zinc-800">
        <div className="flex items-center gap-2">
          <img src="/assets/logo.png" alt="Daedalus" className="w-8 h-8 opacity-70" />
          <div>
            <h1 className="text-sm font-bold tracking-widest text-zinc-300">DAEDALUS</h1>
            <p className="text-[10px] text-zinc-500 tracking-wider">master the labyrinth</p>
          </div>
        </div>
      </div>

      <nav className="flex-1 py-2">
        {navItems.map((item) => (
          <button
            key={item.id}
            onClick={() => setActivePanel(item.id)}
            className={`w-full flex items-center gap-3 px-4 py-2.5 text-sm transition-colors ${
              activePanel === item.id
                ? "bg-zinc-800 text-amber-400 border-r-2 border-amber-400"
                : "text-zinc-400 hover:bg-zinc-800/50 hover:text-zinc-200"
            }`}
          >
            {item.icon}
            {item.label}
          </button>
        ))}
      </nav>

      <div className="p-3 border-t border-zinc-800">
        <button
          onClick={() => setActivePanel("settings")}
          className="w-full flex items-center gap-3 px-3 py-2 text-sm text-zinc-500 hover:text-zinc-300 transition-colors rounded"
        >
          <Settings size={18} />
          Settings
        </button>
        <div className="flex items-center gap-2 px-3 mt-2">
          <div
            className={`w-2 h-2 rounded-full ${
              status === "connected"
                ? "bg-green-500"
                : status === "connecting"
                  ? "bg-yellow-500 animate-pulse"
                  : "bg-red-500"
            }`}
          />
          <span className="text-xs text-zinc-500">
            {status === "connected"
              ? "ECU Connected"
              : status === "connecting"
                ? "Connecting..."
                : "Disconnected"}
          </span>
        </div>
      </div>
    </aside>
  );
}
