import { Sidebar } from "./components/layout/Sidebar";
import { Header } from "./components/layout/Header";
import { StatusBar } from "./components/layout/StatusBar";
import { ConnectionPanel } from "./components/connection/ConnectionPanel";
import { DTCViewer } from "./components/dtc/DTCViewer";
import { AIChat } from "./components/ai/AIChat";
import { ReversePanel } from "./components/reverse/ReversePanel";
import { useUIStore } from "./stores/uiStore";

function PlaceholderPanel({ title, description }: { title: string; description: string }) {
  return (
    <div className="flex-1 flex items-center justify-center">
      <div className="text-center">
        <h3 className="text-lg text-zinc-500 mb-2">{title}</h3>
        <p className="text-sm text-zinc-600">{description}</p>
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
      return <PlaceholderPanel title="Live Data Monitor" description="Connect to ECU to view real-time parameters" />;
    case "flash":
      return <PlaceholderPanel title="Flash Read/Write" description="Connect to ECU to read or write firmware" />;
    case "editor":
      return <PlaceholderPanel title="Map Editor" description="Open a binary file to edit calibration maps" />;
    case "settings":
      return <PlaceholderPanel title="Settings" description="AI provider, API keys, language, theme" />;
    default:
      return <PlaceholderPanel title="Daedalus" description="Select a panel from the sidebar" />;
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
