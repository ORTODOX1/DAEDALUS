import { useState } from "react";
import { Plug, PlugZap, RefreshCw } from "lucide-react";
import { useConnectionStore } from "../../stores/connectionStore";

const BAUD_RATES = [250000, 500000, 1000000];

const MOCK_ADAPTERS = [
  { id: "vcan0", name: "Virtual CAN (vcan0)", type: "socketcan" as const, port: "vcan0", available: true },
  { id: "canable", name: "CANable 2.0", type: "usb" as const, port: "COM3", available: false },
  { id: "macchina", name: "Macchina M2", type: "usb" as const, port: "COM5", available: false },
];

export function ConnectionPanel() {
  const { status, adapters, selectedAdapter, baudRate, ecuInfo, error, setStatus, setAdapters, selectAdapter, setBaudRate, setECUInfo, setError } = useConnectionStore();
  const [scanning, setScanning] = useState(false);

  const handleScan = () => {
    setScanning(true);
    setTimeout(() => {
      setAdapters(MOCK_ADAPTERS);
      setScanning(false);
    }, 1000);
  };

  const handleConnect = () => {
    if (!selectedAdapter) return;
    setStatus("connecting");
    setError(null);
    setTimeout(() => {
      setStatus("connected");
      setECUInfo({
        name: "EDC17C46",
        manufacturer: "Bosch",
        processor: "TC1797",
        hwVersion: "HW03",
        swVersion: "SW001",
        protocol: "bdm",
        vehicleType: "truck",
      });
    }, 2000);
  };

  const handleDisconnect = () => {
    setStatus("disconnected");
    setECUInfo(null);
  };

  const displayAdapters = adapters.length > 0 ? adapters : MOCK_ADAPTERS;

  return (
    <div className="p-6 max-w-2xl">
      <h3 className="text-lg font-semibold text-zinc-200 mb-6">CAN / K-Line Connection</h3>

      {/* Adapter selection */}
      <div className="space-y-4">
        <div className="flex items-center gap-3">
          <h4 className="text-sm font-medium text-zinc-400">Adapters</h4>
          <button
            onClick={handleScan}
            disabled={scanning}
            className="flex items-center gap-1.5 px-3 py-1 text-xs bg-zinc-800 text-zinc-300 rounded hover:bg-zinc-700 transition-colors disabled:opacity-50"
          >
            <RefreshCw size={12} className={scanning ? "animate-spin" : ""} />
            {scanning ? "Scanning..." : "Scan"}
          </button>
        </div>

        <div className="space-y-2">
          {displayAdapters.map((adapter) => (
            <button
              key={adapter.id}
              onClick={() => selectAdapter(adapter.id)}
              className={`w-full flex items-center gap-3 p-3 rounded-lg border transition-colors ${
                selectedAdapter === adapter.id
                  ? "border-amber-500/50 bg-amber-500/10"
                  : "border-zinc-800 bg-zinc-900 hover:border-zinc-700"
              } ${!adapter.available ? "opacity-40" : ""}`}
              disabled={!adapter.available}
            >
              <Plug size={16} className={adapter.available ? "text-green-400" : "text-zinc-600"} />
              <div className="text-left">
                <div className="text-sm text-zinc-200">{adapter.name}</div>
                <div className="text-xs text-zinc-500">{adapter.type.toUpperCase()} — {adapter.port}</div>
              </div>
              {!adapter.available && <span className="ml-auto text-xs text-zinc-600">Not found</span>}
            </button>
          ))}
        </div>

        {/* Baud rate */}
        <div>
          <label className="text-sm text-zinc-400 mb-2 block">Baud Rate</label>
          <div className="flex gap-2">
            {BAUD_RATES.map((rate) => (
              <button
                key={rate}
                onClick={() => setBaudRate(rate)}
                className={`px-4 py-2 text-sm rounded transition-colors ${
                  baudRate === rate
                    ? "bg-amber-500/20 text-amber-400 border border-amber-500/30"
                    : "bg-zinc-800 text-zinc-400 border border-zinc-700 hover:border-zinc-600"
                }`}
              >
                {rate / 1000}k
              </button>
            ))}
          </div>
        </div>

        {/* Connect button */}
        <div className="pt-4">
          {status === "connected" ? (
            <button
              onClick={handleDisconnect}
              className="flex items-center gap-2 px-6 py-2.5 bg-red-500/20 text-red-400 border border-red-500/30 rounded-lg hover:bg-red-500/30 transition-colors"
            >
              <PlugZap size={16} />
              Disconnect
            </button>
          ) : (
            <button
              onClick={handleConnect}
              disabled={!selectedAdapter || status === "connecting"}
              className="flex items-center gap-2 px-6 py-2.5 bg-green-500/20 text-green-400 border border-green-500/30 rounded-lg hover:bg-green-500/30 transition-colors disabled:opacity-30"
            >
              <Plug size={16} />
              {status === "connecting" ? "Connecting..." : "Connect"}
            </button>
          )}
        </div>

        {error && <p className="text-sm text-red-400 mt-2">{error}</p>}

        {/* ECU Info */}
        {ecuInfo && (
          <div className="mt-6 p-4 bg-zinc-900 rounded-lg border border-zinc-800">
            <h4 className="text-sm font-medium text-green-400 mb-3">ECU Identified</h4>
            <div className="grid grid-cols-2 gap-2 text-sm">
              <span className="text-zinc-500">Name:</span>
              <span className="text-zinc-200">{ecuInfo.name}</span>
              <span className="text-zinc-500">Manufacturer:</span>
              <span className="text-zinc-200">{ecuInfo.manufacturer}</span>
              <span className="text-zinc-500">Processor:</span>
              <span className="text-zinc-200">{ecuInfo.processor}</span>
              <span className="text-zinc-500">HW Version:</span>
              <span className="text-zinc-200">{ecuInfo.hwVersion}</span>
              <span className="text-zinc-500">Protocol:</span>
              <span className="text-zinc-200 uppercase">{ecuInfo.protocol}</span>
              <span className="text-zinc-500">Vehicle:</span>
              <span className="text-zinc-200 capitalize">{ecuInfo.vehicleType}</span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
