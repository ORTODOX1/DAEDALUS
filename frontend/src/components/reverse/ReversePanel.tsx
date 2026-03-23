import { useState } from "react";
import { Search, Zap, CircleDot, Cpu, Bot, CheckCircle2, AlertTriangle } from "lucide-react";
import { useReverseStore } from "../../stores/reverseStore";
import type { MultimeterReading, REAgentMessage } from "../../types";

const ECU_LIST = [
  { id: "edc17c46", name: "Bosch EDC17C46", vehicles: "MAN TGA/TGX/TGS", processor: "TC1797" },
  { id: "edc17c49", name: "Bosch EDC17C49", vehicles: "DAF CF/XF", processor: "TC1797" },
  { id: "edc17cv44", name: "Bosch EDC17CV44", vehicles: "Mercedes Actros", processor: "TC1797" },
  { id: "md1cs004", name: "Bosch MD1CS004", vehicles: "MAN TGX (Euro 6d)", processor: "TC297" },
  { id: "cm2350", name: "Cummins CM2350", vehicles: "КамАЗ / PACCAR", processor: "TC1797" },
  { id: "dcm37", name: "Delphi DCM3.7", vehicles: "DAF / Ford Cargo", processor: "MPC5674F" },
];

const AGENT_ROLES = [
  { role: "Voltage Analyst", icon: "V", color: "text-yellow-400" },
  { role: "Signal Tracer", icon: "S", color: "text-blue-400" },
  { role: "Protocol Expert", icon: "P", color: "text-purple-400" },
  { role: "Cross-ECU Analyst", icon: "X", color: "text-green-400" },
  { role: "Safety Checker", icon: "!", color: "text-red-400" },
  { role: "Datasheet Parser", icon: "D", color: "text-cyan-400" },
  { role: "Pattern Matcher", icon: "M", color: "text-orange-400" },
  { role: "Confidence Scorer", icon: "%", color: "text-emerald-400" },
];

export function ReversePanel() {
  const {
    multimeterConnected, selectedECU, readings, agentMessages,
    analysisRunning, setMultimeterConnected, selectECU,
    addReading, addAgentMessage, setAnalysisRunning, clearAgentMessages,
  } = useReverseStore();

  const [mmPort, setMmPort] = useState("COM4");

  const handleConnectMM = () => {
    setMultimeterConnected(!multimeterConnected);
  };

  const handleMeasure = (type: "voltage" | "resistance" | "continuity") => {
    const mockValues = {
      voltage: { value: [3.3, 0.0, 0.02, 1.8, 0.0, 0.01, 0.0][readings.length % 7], unit: "V" },
      resistance: { value: [0.5, 10000, 47000, 100, 0.3, 1000000, 330][readings.length % 7], unit: "Ohm" },
      continuity: { value: readings.length % 3 === 0 ? 1 : 0, unit: "bool" },
    };
    const reading: MultimeterReading = {
      id: crypto.randomUUID(),
      padId: `TP${readings.length + 1}`,
      type,
      value: mockValues[type].value,
      unit: mockValues[type].unit,
      timestamp: Date.now(),
    };
    addReading(reading);
  };

  const handleAnalyze = () => {
    if (readings.length < 3) return;
    setAnalysisRunning(true);
    clearAgentMessages();

    const mockAnalysis: REAgentMessage[] = [
      { agentRole: "Voltage Analyst", content: `TP1 shows 3.3V — typical VCC for ${selectedECU || "TC1797"} processor core. High confidence this is the power supply test pad.`, confidence: 0.95, round: 1, phase: "generate" },
      { agentRole: "Signal Tracer", content: "TP2 measures 0.0V with very low resistance to ground — confirmed GND pad. Adjacent to TP1 which is consistent with typical VCC/GND pair placement.", confidence: 0.98, round: 1, phase: "generate" },
      { agentRole: "Datasheet Parser", content: "TC1797 BDM interface requires: VCC(3.3V), GND, TCK, TDI, TDO, TMS, RESET. Pin 234=TCK, Pin 235=TDI, Pin 236=TDO per Infineon datasheet.", confidence: 0.90, round: 1, phase: "generate" },
      { agentRole: "Cross-ECU Analyst", content: "EDC17C46 HW03 layout matches known EDC17C49 pattern — test pads are in same relative positions. TP3 position corresponds to TCK on the C49 reference board.", confidence: 0.85, round: 2, phase: "generate" },
      { agentRole: "Safety Checker", content: "WARNING: TP1 (VCC 3.3V) — do NOT apply external voltage higher than 3.6V. TC1797 I/O maximum is 3.63V. Exceeding this will damage the processor permanently.", confidence: 1.0, round: 2, phase: "critique" },
      { agentRole: "Protocol Expert", content: "Signal set matches BDM protocol (not JTAG). TC1797 uses Nexus/OCDS debug interface via BDM. Required signals: BRKOUT, BKPT, TCK, TDI, TDO — 5 pads minimum.", confidence: 0.92, round: 2, phase: "generate" },
      { agentRole: "Pattern Matcher", content: "Test pad cluster at (45-55, 115-130) matches Bosch EDC17 standard layout revision 3+. Pads arranged in L-shape near processor, consistent with 14 other verified EDC17 variants.", confidence: 0.88, round: 3, phase: "vote" },
      { agentRole: "Confidence Scorer", content: "FINAL ASSESSMENT:\n• TP1 = VCC (3.3V) — 95%\n• TP2 = GND — 98%\n• TP3 = TCK — 87%\n• TP4 = TDI — 72% (needs continuity check to pin 235)\n• TP5 = TDO — 70%\n• Protocol: BDM — 92%", confidence: 0.88, round: 3, phase: "vote" },
    ];

    let i = 0;
    const interval = setInterval(() => {
      if (i < mockAnalysis.length) {
        addAgentMessage(mockAnalysis[i]);
        i++;
      } else {
        clearInterval(interval);
        setAnalysisRunning(false);
      }
    }, 1200);
  };

  return (
    <div className="h-full flex">
      {/* Left: Controls */}
      <div className="w-80 border-r border-zinc-800 flex flex-col overflow-auto">
        {/* ECU Selection */}
        <div className="p-4 border-b border-zinc-800">
          <h4 className="text-sm font-medium text-zinc-400 mb-2">Select ECU Type</h4>
          <div className="space-y-1.5 max-h-40 overflow-auto">
            {ECU_LIST.map((ecu) => (
              <button
                key={ecu.id}
                onClick={() => selectECU(ecu.id)}
                className={`w-full text-left px-3 py-2 rounded text-xs transition-colors ${
                  selectedECU === ecu.id
                    ? "bg-amber-500/15 text-amber-400 border border-amber-500/30"
                    : "bg-zinc-900 text-zinc-400 border border-zinc-800 hover:border-zinc-700"
                }`}
              >
                <div className="font-medium">{ecu.name}</div>
                <div className="text-zinc-500">{ecu.vehicles} — {ecu.processor}</div>
              </button>
            ))}
          </div>
        </div>

        {/* Multimeter */}
        <div className="p-4 border-b border-zinc-800">
          <h4 className="text-sm font-medium text-zinc-400 mb-2">Multimeter</h4>
          <div className="flex gap-2 mb-3">
            <input
              type="text"
              value={mmPort}
              onChange={(e) => setMmPort(e.target.value)}
              placeholder="COM4 / /dev/ttyUSB0"
              className="flex-1 px-3 py-1.5 bg-zinc-900 border border-zinc-800 rounded text-xs text-zinc-200 focus:outline-none focus:border-zinc-600"
            />
            <button
              onClick={handleConnectMM}
              className={`px-3 py-1.5 text-xs rounded transition-colors ${
                multimeterConnected
                  ? "bg-green-500/20 text-green-400"
                  : "bg-zinc-800 text-zinc-400 hover:bg-zinc-700"
              }`}
            >
              {multimeterConnected ? "Connected" : "Connect"}
            </button>
          </div>
          <div className="flex gap-2">
            <button onClick={() => handleMeasure("voltage")} className="flex-1 flex items-center justify-center gap-1 px-2 py-2 bg-yellow-500/10 text-yellow-400 border border-yellow-500/20 rounded text-xs hover:bg-yellow-500/20 transition-colors">
              <Zap size={12} /> Voltage
            </button>
            <button onClick={() => handleMeasure("resistance")} className="flex-1 flex items-center justify-center gap-1 px-2 py-2 bg-blue-500/10 text-blue-400 border border-blue-500/20 rounded text-xs hover:bg-blue-500/20 transition-colors">
              <CircleDot size={12} /> Resist
            </button>
            <button onClick={() => handleMeasure("continuity")} className="flex-1 flex items-center justify-center gap-1 px-2 py-2 bg-green-500/10 text-green-400 border border-green-500/20 rounded text-xs hover:bg-green-500/20 transition-colors">
              <Search size={12} /> Beep
            </button>
          </div>
        </div>

        {/* Readings */}
        <div className="p-4 flex-1 overflow-auto">
          <h4 className="text-sm font-medium text-zinc-400 mb-2">Measurements ({readings.length})</h4>
          {readings.length === 0 ? (
            <p className="text-xs text-zinc-600">No readings yet. Place probe and click measure.</p>
          ) : (
            <div className="space-y-1">
              {readings.map((r) => (
                <div key={r.id} className="flex items-center gap-2 px-2 py-1.5 bg-zinc-900 rounded text-xs">
                  <span className="font-mono text-amber-400 w-8">{r.padId}</span>
                  <span className="text-zinc-400 w-16">{r.type}</span>
                  <span className="font-mono text-zinc-200">
                    {r.type === "continuity" ? (r.value ? "YES" : "NO") : `${r.value} ${r.unit}`}
                  </span>
                </div>
              ))}
            </div>
          )}

          {readings.length >= 3 && (
            <button
              onClick={handleAnalyze}
              disabled={analysisRunning}
              className="w-full mt-3 flex items-center justify-center gap-2 px-4 py-2.5 bg-amber-500/20 text-amber-400 border border-amber-500/30 rounded-lg hover:bg-amber-500/30 transition-colors disabled:opacity-50"
            >
              <Cpu size={14} />
              {analysisRunning ? "Analyzing..." : "AI Analyze Pinout"}
            </button>
          )}
        </div>
      </div>

      {/* Right: AI Agent Stream */}
      <div className="flex-1 flex flex-col overflow-auto">
        <div className="p-4 border-b border-zinc-800 flex items-center gap-3">
          <Bot size={18} className="text-amber-400" />
          <h4 className="text-sm font-medium text-zinc-200">AI Reverse Engineering Agents</h4>
          <div className="flex gap-1 ml-auto">
            {AGENT_ROLES.map((a) => (
              <span key={a.role} className={`w-5 h-5 text-[10px] flex items-center justify-center rounded bg-zinc-800 ${a.color}`} title={a.role}>
                {a.icon}
              </span>
            ))}
          </div>
        </div>

        <div className="flex-1 overflow-auto p-4 space-y-3">
          {agentMessages.length === 0 && !analysisRunning && (
            <div className="text-center text-zinc-600 text-sm py-12">
              <Cpu size={40} className="mx-auto mb-3 opacity-30" />
              <p>Select ECU, take measurements, then click "AI Analyze Pinout"</p>
              <p className="text-xs mt-1">8 AI agents will debate and determine the pinout</p>
            </div>
          )}

          {agentMessages.map((msg, i) => {
            const role = AGENT_ROLES.find((r) => r.role === msg.agentRole);
            return (
              <div key={i} className="flex gap-3">
                <div className={`w-8 h-8 rounded-lg bg-zinc-800 flex items-center justify-center text-xs font-bold flex-shrink-0 ${role?.color ?? "text-zinc-400"}`}>
                  {role?.icon ?? "?"}
                </div>
                <div className="flex-1 bg-zinc-900 rounded-lg p-3 border border-zinc-800">
                  <div className="flex items-center gap-2 mb-1">
                    <span className={`text-xs font-medium ${role?.color ?? "text-zinc-400"}`}>{msg.agentRole}</span>
                    <span className="text-[10px] text-zinc-600">Round {msg.round} — {msg.phase}</span>
                    <span className="ml-auto text-[10px] text-zinc-600">{Math.round(msg.confidence * 100)}% conf</span>
                  </div>
                  <p className="text-sm text-zinc-300 whitespace-pre-wrap">{msg.content}</p>
                </div>
              </div>
            );
          })}

          {analysisRunning && (
            <div className="flex items-center gap-2 text-sm text-amber-400 animate-pulse">
              <div className="w-2 h-2 rounded-full bg-amber-400 animate-ping" />
              Agents debating...
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
