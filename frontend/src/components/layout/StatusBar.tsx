import { useConnectionStore } from "../../stores/connectionStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useReverseStore } from "../../stores/reverseStore";

export function StatusBar() {
  const status = useConnectionStore((s) => s.status);
  const baudRate = useConnectionStore((s) => s.baudRate);
  const provider = useSettingsStore((s) => s.aiProvider);
  const mmConnected = useReverseStore((s) => s.multimeterConnected);

  return (
    <footer className="h-7 bg-zinc-900 border-t border-zinc-800 flex items-center px-4 gap-6 text-[11px] text-zinc-500">
      <div className="flex items-center gap-1.5">
        <div
          className={`w-1.5 h-1.5 rounded-full ${
            status === "connected" ? "bg-green-500" : "bg-zinc-600"
          }`}
        />
        CAN: {status === "connected" ? `${baudRate / 1000}k` : "---"}
      </div>

      <div>AI: {provider.type}/{provider.model.split("-").slice(0, 2).join("-")}</div>

      <div className="flex items-center gap-1.5">
        <div
          className={`w-1.5 h-1.5 rounded-full ${mmConnected ? "bg-blue-500" : "bg-zinc-600"}`}
        />
        Multimeter: {mmConnected ? "OK" : "---"}
      </div>

      <div className="flex-1" />
      <div>Daedalus ECU Tuning Platform</div>
    </footer>
  );
}
