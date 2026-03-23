import { useState, useCallback, useRef, useEffect } from "react";
import {
  Shield,
  Zap,
  Gauge,
  AlertTriangle,
  Brain,
  Play,
  RotateCcw,
  CheckCircle2,
  XCircle,
} from "lucide-react";

/* --- Types --- */

interface AgentConfig {
  id: string;
  name: string;
  role: string;
  color: string;
  bgColor: string;
  borderColor: string;
  icon: typeof Shield;
}

interface AgentMessage {
  agentId: string;
  round: number;
  phase: "generate" | "critique" | "vote";
  content: string;
  confidence: number;
}

type Verdict = "SAFE" | "CAUTION" | "BLOCKED";

interface DebateResult {
  verdict: Verdict;
  confidence: number;
  recommendation: string;
}

/* --- Constants --- */

const AGENTS: AgentConfig[] = [
  { id: "stock",        name: "Stock Defender",       role: "Argues for factory settings",    color: "text-blue-400",   bgColor: "bg-blue-500/10",   borderColor: "border-blue-500/20", icon: Shield },
  { id: "conservative", name: "Conservative Tuner",   role: "Careful optimization",           color: "text-green-400",  bgColor: "bg-green-500/10",  borderColor: "border-green-500/20", icon: CheckCircle2 },
  { id: "aggressive",   name: "Aggressive Optimizer", role: "Maximum performance",            color: "text-orange-400", bgColor: "bg-orange-500/10", borderColor: "border-orange-500/20", icon: Zap },
  { id: "skeptic",      name: "Safety Skeptic",       role: "Finds dangers",                  color: "text-red-400",    bgColor: "bg-red-500/10",    borderColor: "border-red-500/20",   icon: AlertTriangle },
  { id: "ml",           name: "ML Predictor",         role: "Data-driven analysis",           color: "text-purple-400", bgColor: "bg-purple-500/10", borderColor: "border-purple-500/20", icon: Brain },
];

const ROUND_LABELS = ["Round 1: Generate", "Round 2: Critique", "Round 3: Vote"];

/* --- Mock debate content --- */

function getMockMessages(modification: string): AgentMessage[][] {
  const mod = modification.toLowerCase();
  const isBoost = mod.includes("boost");
  const isTiming = mod.includes("timing");

  return [
    // Round 1: Generate
    [
      { agentId: "stock", round: 1, phase: "generate", confidence: 85,
        content: isBoost
          ? "Factory boost curve is calibrated for 250,000 km durability. OEM spent 18 months validating thermal limits at every RPM point. Any increase risks turbo bearing wear and intercooler saturation."
          : "Factory calibration ensures optimal balance between performance, emissions, and component longevity. Modifying this parameter deviates from validated operating envelope." },
      { agentId: "conservative", round: 1, phase: "generate", confidence: 72,
        content: isBoost
          ? "Can safely add +0.15-0.2 bar if we verify intercooler efficiency is >85% at target RPM. Need to check compressor map — we might be near surge line at low RPM with +0.3 bar."
          : "A modest change of 50-60% of the requested delta is feasible with proper validation. Recommend staged approach with data logging between steps." },
      { agentId: "aggressive", round: 1, phase: "generate", confidence: 90,
        content: isBoost
          ? "+0.3 bar is well within turbo specification margins. Modern wastegate actuators handle this easily. The factory left 15-20% headroom for altitude compensation that we can reclaim at sea level."
          : "The requested modification is within the component's design envelope. Factory derating for worst-case scenarios leaves significant margin for optimized conditions." },
      { agentId: "skeptic", round: 1, phase: "generate", confidence: 40,
        content: isBoost
          ? "At 40\u00B0C ambient, intercooler delta-T drops. Combined with +0.3 bar, charge air temp could exceed 65\u00B0C, pushing knock threshold. Also: turbo oil feed line stress increases non-linearly above OEM spec boost."
          : "Worst-case scenario analysis: component stress increases exponentially near design limits. Failure modes include thermal fatigue, excessive wear, and cascading damage to adjacent systems." },
      { agentId: "ml", round: 1, phase: "generate", confidence: 78,
        content: isBoost
          ? "Dataset of 1,247 similar ECUs shows: +0.2 bar = 0.3% failure rate at 100k km. +0.3 bar = 2.1% failure rate. Inflection point is at +0.25 bar for this turbo model. Recommend staying below inflection."
          : "Statistical analysis of 1,247 comparable modifications shows acceptable risk at 70% of requested delta. Full delta pushes failure probability above 2% threshold." },
    ],
    // Round 2: Critique
    [
      { agentId: "stock", round: 2, phase: "critique", confidence: 80,
        content: "ML data confirms my concern. 2.1% failure rate means 1 in 50 engines. That is unacceptable for a commercial vehicle. Conservative tuner's +0.2 bar suggestion aligns with safe zone." },
      { agentId: "conservative", round: 2, phase: "critique", confidence: 75,
        content: "Aggressive's point about altitude margin is valid, but only for sea-level operation. Skeptic raises good thermal concerns. I stand by +0.2 bar with mandatory EGT monitoring during first 500 km." },
      { agentId: "aggressive", round: 2, phase: "critique", confidence: 70,
        content: "ML data shows 0.3% at +0.2 bar is essentially zero risk. Even +0.3 bar at 2.1% is acceptable for performance applications. But I concede the thermal argument — add charge air temp safety cutback." },
      { agentId: "skeptic", round: 2, phase: "critique", confidence: 55,
        content: "Aggressive now agrees thermal management is needed. If we add a boost cutback at charge air >60\u00B0C, the +0.2 bar proposal becomes defensible. Still concerned about long-term turbo bearing wear." },
      { agentId: "ml", round: 2, phase: "critique", confidence: 82,
        content: "Updating prediction with thermal safeguard: +0.2 bar with EGT/charge temp cutback reduces failure rate to 0.15%. +0.3 bar with same safeguards: 1.4%. Clear recommendation for +0.2 bar." },
    ],
    // Round 3: Vote
    [
      { agentId: "stock", round: 3, phase: "vote", confidence: 70,
        content: "VOTE: CAUTION. Accept +0.2 bar with thermal safeguards. +0.3 bar is too aggressive for the risk profile." },
      { agentId: "conservative", round: 3, phase: "vote", confidence: 80,
        content: "VOTE: CAUTION. +0.2 bar is safe with monitoring. Recommend EGT cutback at 780\u00B0C and charge air cutback at 60\u00B0C." },
      { agentId: "aggressive", round: 3, phase: "vote", confidence: 65,
        content: "VOTE: CAUTION. Revised down to +0.2 bar based on ML data. Add thermal safeguards for full safety margin." },
      { agentId: "skeptic", round: 3, phase: "vote", confidence: 60,
        content: "VOTE: CAUTION. Conditionally accept +0.2 bar. Require data logging for first 1000 km and re-evaluation." },
      { agentId: "ml", round: 3, phase: "vote", confidence: 85,
        content: "VOTE: CAUTION. Statistical confidence for +0.2 bar with safeguards: 96.3% safe. Recommended implementation with monitoring." },
    ],
  ];
}

/* --- Component --- */

export function SafetyDebatePanel() {
  const [modification, setModification] = useState("Boost +0.3 bar at 2000 RPM");
  const [messages, setMessages] = useState<AgentMessage[]>([]);
  const [debateRunning, setDebateRunning] = useState(false);
  const [currentRound, setCurrentRound] = useState(0);
  const [result, setResult] = useState<DebateResult | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const abortRef = useRef(false);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight, behavior: "smooth" });
  }, [messages]);

  const runDebate = useCallback(async () => {
    if (!modification.trim()) return;
    setMessages([]);
    setResult(null);
    setDebateRunning(true);
    setCurrentRound(0);
    abortRef.current = false;

    const allRounds = getMockMessages(modification);

    for (let round = 0; round < allRounds.length; round++) {
      if (abortRef.current) break;
      setCurrentRound(round + 1);

      for (const msg of allRounds[round]) {
        if (abortRef.current) break;
        await new Promise((r) => setTimeout(r, 800));
        setMessages((prev) => [...prev, msg]);
      }

      if (round < allRounds.length - 1) {
        await new Promise((r) => setTimeout(r, 600));
      }
    }

    if (!abortRef.current) {
      setResult({
        verdict: "CAUTION",
        confidence: 72,
        recommendation: "Reduce to +0.2 bar with EGT cutback at 780\u00B0C and charge air cutback at 60\u00B0C. Require data logging for first 1000 km.",
      });
    }

    setDebateRunning(false);
  }, [modification]);

  const handleReset = () => {
    abortRef.current = true;
    setMessages([]);
    setResult(null);
    setDebateRunning(false);
    setCurrentRound(0);
  };

  const getAgent = (id: string) => AGENTS.find((a) => a.id === id)!;

  const verdictStyles: Record<Verdict, { bg: string; border: string; text: string; icon: typeof CheckCircle2 }> = {
    SAFE:    { bg: "bg-green-500/10",  border: "border-green-500/30", text: "text-green-400",  icon: CheckCircle2 },
    CAUTION: { bg: "bg-yellow-500/10", border: "border-yellow-500/30", text: "text-yellow-400", icon: AlertTriangle },
    BLOCKED: { bg: "bg-red-500/10",    border: "border-red-500/30",   text: "text-red-400",    icon: XCircle },
  };

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="p-4 border-b border-zinc-800 space-y-3">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-amber-500/10 flex items-center justify-center">
            <Gauge size={16} className="text-amber-400" />
          </div>
          <div>
            <h2 className="text-sm font-semibold text-zinc-200">Safety Debate</h2>
            <p className="text-xs text-zinc-500">5 AI agents evaluate proposed modification</p>
          </div>
        </div>

        <div className="flex gap-2">
          <input
            type="text"
            value={modification}
            onChange={(e) => setModification(e.target.value)}
            placeholder="Describe modification, e.g. Boost +0.3 bar at 2000 RPM"
            disabled={debateRunning}
            className="flex-1 px-3 py-2 bg-zinc-900 border border-zinc-700 rounded-lg text-sm text-zinc-200 placeholder:text-zinc-600 focus:outline-none focus:border-amber-500/50 disabled:opacity-50"
          />
          {debateRunning ? (
            <button
              onClick={handleReset}
              className="flex items-center gap-2 px-4 py-2 bg-red-500/10 text-red-400 border border-red-500/20 rounded-lg text-sm hover:bg-red-500/20 transition-colors"
            >
              <RotateCcw size={14} />
              Stop
            </button>
          ) : (
            <button
              onClick={runDebate}
              disabled={!modification.trim()}
              className="flex items-center gap-2 px-4 py-2 bg-amber-500/10 text-amber-400 border border-amber-500/20 rounded-lg text-sm hover:bg-amber-500/20 transition-colors disabled:opacity-30"
            >
              <Play size={14} />
              Run Debate
            </button>
          )}
        </div>

        {/* Agent legend */}
        <div className="flex flex-wrap gap-2">
          {AGENTS.map((agent) => (
            <div key={agent.id} className={`flex items-center gap-1.5 px-2 py-1 rounded text-[10px] ${agent.bgColor} ${agent.color} border ${agent.borderColor}`}>
              <agent.icon size={10} />
              {agent.name}
            </div>
          ))}
        </div>

        {/* Round progress */}
        {(debateRunning || messages.length > 0) && (
          <div className="flex gap-2">
            {ROUND_LABELS.map((label, i) => (
              <div
                key={label}
                className={`flex-1 text-center py-1.5 rounded text-[10px] font-medium border transition-colors ${
                  currentRound > i + 1 || result
                    ? "bg-green-500/10 border-green-500/20 text-green-400"
                    : currentRound === i + 1
                      ? "bg-amber-500/10 border-amber-500/20 text-amber-400"
                      : "bg-zinc-900 border-zinc-800 text-zinc-600"
                }`}
              >
                {label}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Messages area */}
      <div ref={scrollRef} className="flex-1 overflow-auto p-4 space-y-3">
        {messages.length === 0 && !debateRunning && (
          <div className="h-full flex items-center justify-center">
            <div className="text-center space-y-3">
              <Gauge size={40} className="text-zinc-700 mx-auto" />
              <p className="text-sm text-zinc-600">
                Enter a proposed modification and click "Run Debate"
              </p>
              <p className="text-xs text-zinc-700 max-w-md">
                5 AI agents with different perspectives will evaluate the safety of your modification in 3 rounds: Generate, Critique, and Vote.
              </p>
            </div>
          </div>
        )}

        {messages.map((msg, idx) => {
          const agent = getAgent(msg.agentId);
          const Icon = agent.icon;

          return (
            <div
              key={idx}
              className={`flex gap-3 animate-in fade-in slide-in-from-bottom-2 duration-300`}
            >
              <div className={`w-8 h-8 rounded-lg ${agent.bgColor} flex items-center justify-center flex-shrink-0 border ${agent.borderColor}`}>
                <Icon size={14} className={agent.color} />
              </div>
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-1">
                  <span className={`text-xs font-semibold ${agent.color}`}>{agent.name}</span>
                  <span className="text-[10px] text-zinc-600 uppercase">{msg.phase}</span>
                  <div className="flex items-center gap-1 ml-auto">
                    <span className="text-[10px] text-zinc-500">confidence:</span>
                    <span className={`text-[10px] font-mono font-bold ${
                      msg.confidence >= 75 ? "text-green-400" : msg.confidence >= 50 ? "text-yellow-400" : "text-red-400"
                    }`}>
                      {msg.confidence}%
                    </span>
                  </div>
                </div>
                <p className="text-sm text-zinc-300 leading-relaxed">{msg.content}</p>
              </div>
            </div>
          );
        })}

        {debateRunning && (
          <div className="flex gap-3 items-center">
            <div className="w-8 h-8 rounded-lg bg-zinc-800 flex items-center justify-center">
              <Brain size={14} className="text-zinc-500 animate-pulse" />
            </div>
            <div className="flex gap-1">
              <div className="w-2 h-2 bg-zinc-600 rounded-full animate-bounce" />
              <div className="w-2 h-2 bg-zinc-600 rounded-full animate-bounce" style={{ animationDelay: "0.15s" }} />
              <div className="w-2 h-2 bg-zinc-600 rounded-full animate-bounce" style={{ animationDelay: "0.3s" }} />
            </div>
          </div>
        )}
      </div>

      {/* Verdict */}
      {result && (
        <div className={`mx-4 mb-4 p-4 rounded-xl border ${verdictStyles[result.verdict].bg} ${verdictStyles[result.verdict].border}`}>
          <div className="flex items-center gap-3 mb-2">
            {(() => { const VIcon = verdictStyles[result.verdict].icon; return <VIcon size={20} className={verdictStyles[result.verdict].text} />; })()}
            <span className={`text-lg font-bold ${verdictStyles[result.verdict].text}`}>
              {result.verdict}
            </span>
            <span className="text-sm text-zinc-400">
              ({result.confidence}% safe)
            </span>
          </div>
          <p className="text-sm text-zinc-300">{result.recommendation}</p>
        </div>
      )}
    </div>
  );
}
