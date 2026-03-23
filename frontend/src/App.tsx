import { lazy, Suspense } from "react";
import { Sidebar } from "./components/layout/Sidebar";
import { Header } from "./components/layout/Header";
import { StatusBar } from "./components/layout/StatusBar";
import { ConnectionPanel } from "./components/connection/ConnectionPanel";
import { DTCViewer } from "./components/dtc/DTCViewer";
import { AIChat } from "./components/ai/AIChat";
import { ReversePanel } from "./components/reverse/ReversePanel";
import { useUIStore } from "./stores/uiStore";

// Lazy load heavy components
const HexEditor = lazy(() => import("./components/editor/HexEditor").then(m => ({ default: m.HexEditor })));
const MapEditor2D = lazy(() => import("./components/editor/MapEditor2D").then(m => ({ default: m.MapEditor2D })));
const DiffView = lazy(() => import("./components/editor/DiffView").then(m => ({ default: m.DiffView })));
const LiveDataPanel = lazy(() => import("./components/live/LiveDataPanel").then(m => ({ default: m.LiveDataPanel })));
const SettingsPanel = lazy(() => import("./components/common/SettingsPanel").then(m => ({ default: m.SettingsPanel })));
const SafetyDebatePanel = lazy(() => import("./components/ai/SafetyDebatePanel").then(m => ({ default: m.SafetyDebatePanel })));
const WhatIfSimulator = lazy(() => import("./components/ai/WhatIfSimulator").then(m => ({ default: m.WhatIfSimulator })));

function Loading() {
  return (
    <div className="flex-1 flex items-center justify-center">
      <div className="text-zinc-500 animate-pulse">Loading...</div>
    </div>
  );
}

function FlashPanel() {
  return (
    <div className="p-6 space-y-6">
      <h3 className="text-lg font-semibold text-zinc-200">Flash Read/Write</h3>
      <div className="grid grid-cols-2 gap-4 max-w-2xl">
        <button className="p-6 bg-zinc-900 border border-zinc-800 rounded-lg hover:border-blue-500/30 transition-colors text-left">
          <div className="text-blue-400 font-medium mb-1">Read ECU</div>
          <div className="text-xs text-zinc-500">Read firmware from connected ECU via UDS/BDM</div>
        </button>
        <button className="p-6 bg-zinc-900 border border-zinc-800 rounded-lg hover:border-amber-500/30 transition-colors text-left">
          <div className="text-amber-400 font-medium mb-1">Write ECU</div>
          <div className="text-xs text-zinc-500">Write modified firmware with safety checks</div>
        </button>
        <button className="p-6 bg-zinc-900 border border-zinc-800 rounded-lg hover:border-green-500/30 transition-colors text-left">
          <div className="text-green-400 font-medium mb-1">Verify Checksum</div>
          <div className="text-xs text-zinc-500">CRC32, Bosch ME7/MED17 multipoint</div>
        </button>
        <button className="p-6 bg-zinc-900 border border-zinc-800 rounded-lg hover:border-purple-500/30 transition-colors text-left">
          <div className="text-purple-400 font-medium mb-1">Backup Manager</div>
          <div className="text-xs text-zinc-500">View and restore firmware backups</div>
        </button>
      </div>
    </div>
  );
}

function EditorPanel() {
  const [tab, setTab] = useUIStore((s) => [s.editorTab ?? "hex", s.setEditorTab ?? (() => {})]);

  return (
    <div className="h-full flex flex-col">
      <div className="flex border-b border-zinc-800">
        {(["hex", "map2d", "diff", "safety", "whatif"] as const).map((t) => (
          <button
            key={t}
            onClick={() => typeof setTab === 'function' && setTab(t)}
            className={`px-4 py-2 text-sm transition-colors ${
              tab === t
                ? "text-amber-400 border-b-2 border-amber-400 bg-zinc-900"
                : "text-zinc-500 hover:text-zinc-300"
            }`}
          >
            {{ hex: "Hex Editor", map2d: "Map Editor", diff: "Diff View", safety: "Safety Debate", whatif: "What-If" }[t]}
          </button>
        ))}
      </div>
      <div className="flex-1 overflow-hidden">
        <Suspense fallback={<Loading />}>
          {tab === "hex" && <HexEditor />}
          {tab === "map2d" && <MapEditor2D />}
          {tab === "diff" && <DiffView />}
          {tab === "safety" && <SafetyDebatePanel />}
          {tab === "whatif" && <WhatIfSimulator />}
        </Suspense>
      </div>
    </div>
  );
}

function MainContent() {
  const activePanel = useUIStore((s) => s.activePanel);

  switch (activePanel) {
    case "connection":
      return <ConnectionPanel />;
    case "dtc":
      return <DTCViewer />;
    case "ai":
      return <AIChat />;
    case "reverse":
      return <ReversePanel />;
    case "live":
      return (
        <Suspense fallback={<Loading />}>
          <LiveDataPanel />
        </Suspense>
      );
    case "flash":
      return <FlashPanel />;
    case "editor":
      return <EditorPanel />;
    case "settings":
      return (
        <Suspense fallback={<Loading />}>
          <SettingsPanel />
        </Suspense>
      );
    default:
      return <EditorPanel />;
  }
}

export default function App() {
  return (
    <div className="flex h-screen bg-zinc-950 text-zinc-100 overflow-hidden">
      <Sidebar />
      <div className="flex-1 flex flex-col min-w-0">
        <Header />
        <main className="flex-1 overflow-hidden">
          <MainContent />
        </main>
        <StatusBar />
      </div>
    </div>
  );
}
