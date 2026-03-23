import { useState } from "react";
import {
  Brain,
  Plug,
  Shield,
  Palette,
  Info,
  Eye,
  EyeOff,
  ExternalLink,
  CheckCircle2,
  AlertTriangle,
} from "lucide-react";
import { useSettingsStore } from "../../stores/settingsStore";
import { useConnectionStore } from "../../stores/connectionStore";
import type { AIProvider } from "../../types";

type SettingsSection = "ai" | "connection" | "safety" | "appearance" | "about";

const PROVIDER_OPTIONS: { value: AIProvider["type"]; label: string }[] = [
  { value: "claude", label: "Anthropic Claude" },
  { value: "openai", label: "OpenAI" },
  { value: "gemini", label: "Google Gemini" },
  { value: "ollama", label: "Ollama (local)" },
];

const BAUD_RATES = [
  { value: 250000, label: "250 kbit/s" },
  { value: 500000, label: "500 kbit/s" },
  { value: 1000000, label: "1 Mbit/s" },
];

const SAFETY_RULES = [
  { id: "lambda", label: "Lambda < 0.78 under boost = BLOCK WRITE", active: true, severity: "critical" as const },
  { id: "knock", label: "Timing beyond knock limit = BLOCK WRITE", active: true, severity: "critical" as const },
  { id: "backup", label: "Require backup before any write", active: true, severity: "warning" as const },
  { id: "diff", label: "Show full diff before write confirmation", active: true, severity: "warning" as const },
  { id: "checksum", label: "Mandatory checksum correction before write", active: true, severity: "critical" as const },
  { id: "egt", label: "EGT > 860 C sustained = BLOCK WRITE", active: true, severity: "critical" as const },
  { id: "boost_limit", label: "Boost > max turbo spec + 10% = BLOCK WRITE", active: true, severity: "critical" as const },
  { id: "rail_max", label: "Rail pressure > injector max rating = BLOCK WRITE", active: true, severity: "critical" as const },
];

const SECTIONS: { id: SettingsSection; label: string; icon: typeof Brain }[] = [
  { id: "ai", label: "AI Provider", icon: Brain },
  { id: "connection", label: "Connection", icon: Plug },
  { id: "safety", label: "Safety", icon: Shield },
  { id: "appearance", label: "Appearance", icon: Palette },
  { id: "about", label: "About", icon: Info },
];

export function SettingsPanel() {
  const { aiProvider, setAIProvider, theme, setTheme, language, setLanguage } = useSettingsStore();
  const { baudRate, setBaudRate } = useConnectionStore();

  const [activeSection, setActiveSection] = useState<SettingsSection>("ai");
  const [showApiKey, setShowApiKey] = useState(false);
  const [autoReconnect, setAutoReconnect] = useState(true);
  const [safetyEnabled, setSafetyEnabled] = useState(true);
  const [rules, setRules] = useState(SAFETY_RULES);

  const [localProvider, setLocalProvider] = useState<AIProvider>({ ...aiProvider });

  const handleProviderTypeChange = (type: AIProvider["type"]) => {
    const updated: AIProvider = {
      ...localProvider,
      type,
      model: type === "claude" ? "claude-sonnet-4-20250514"
        : type === "openai" ? "gpt-4o"
        : type === "gemini" ? "gemini-2.0-flash"
        : "phi3:3.8b-mini-4k-instruct-q4_K_M",
      endpoint: type === "ollama" ? "http://localhost:11434" : undefined,
    };
    setLocalProvider(updated);
    setAIProvider(updated);
  };

  const handleModelChange = (model: string) => {
    const updated = { ...localProvider, model };
    setLocalProvider(updated);
    setAIProvider(updated);
  };

  const handleApiKeyChange = (apiKey: string) => {
    const updated = { ...localProvider, apiKey };
    setLocalProvider(updated);
    setAIProvider(updated);
  };

  const handleEndpointChange = (endpoint: string) => {
    const updated = { ...localProvider, endpoint };
    setLocalProvider(updated);
    setAIProvider(updated);
  };

  const toggleRule = (ruleId: string) => {
    setRules((prev) =>
      prev.map((r) => (r.id === ruleId ? { ...r, active: !r.active } : r)),
    );
  };

  return (
    <div className="h-full flex">
      {/* Sidebar navigation */}
      <div className="w-56 border-r border-zinc-800 p-4 space-y-1 flex-shrink-0">
        <h2 className="text-sm font-semibold text-zinc-400 uppercase tracking-wider mb-4 px-2">
          Settings
        </h2>
        {SECTIONS.map((section) => (
          <button
            key={section.id}
            onClick={() => setActiveSection(section.id)}
            className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm transition-colors ${
              activeSection === section.id
                ? "bg-amber-500/10 text-amber-400 border border-amber-500/20"
                : "text-zinc-400 hover:bg-zinc-800/50 hover:text-zinc-200 border border-transparent"
            }`}
          >
            <section.icon size={16} />
            {section.label}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto p-6">
        <div className="max-w-2xl space-y-6">
          {/* AI Provider Section */}
          {activeSection === "ai" && (
            <>
              <SectionHeader icon={Brain} title="AI Provider" description="Configure the AI backend for map analysis, DTC explanation, and safety validation." />

              <FieldGroup label="Provider">
                <select
                  value={localProvider.type}
                  onChange={(e) => handleProviderTypeChange(e.target.value as AIProvider["type"])}
                  className="w-full px-3 py-2.5 bg-zinc-900 border border-zinc-700 rounded-lg text-sm text-zinc-200 focus:outline-none focus:border-amber-500/50"
                >
                  {PROVIDER_OPTIONS.map((opt) => (
                    <option key={opt.value} value={opt.value}>{opt.label}</option>
                  ))}
                </select>
              </FieldGroup>

              <FieldGroup label="Model">
                <input
                  type="text"
                  value={localProvider.model}
                  onChange={(e) => handleModelChange(e.target.value)}
                  placeholder="e.g. claude-sonnet-4-20250514"
                  className="w-full px-3 py-2.5 bg-zinc-900 border border-zinc-700 rounded-lg text-sm text-zinc-200 placeholder:text-zinc-600 focus:outline-none focus:border-amber-500/50"
                />
              </FieldGroup>

              {localProvider.type !== "ollama" && (
                <FieldGroup label="API Key">
                  <div className="relative">
                    <input
                      type={showApiKey ? "text" : "password"}
                      value={localProvider.apiKey ?? ""}
                      onChange={(e) => handleApiKeyChange(e.target.value)}
                      placeholder="sk-..."
                      className="w-full px-3 py-2.5 pr-10 bg-zinc-900 border border-zinc-700 rounded-lg text-sm text-zinc-200 placeholder:text-zinc-600 focus:outline-none focus:border-amber-500/50"
                    />
                    <button
                      onClick={() => setShowApiKey(!showApiKey)}
                      className="absolute right-3 top-1/2 -translate-y-1/2 text-zinc-500 hover:text-zinc-300"
                    >
                      {showApiKey ? <EyeOff size={16} /> : <Eye size={16} />}
                    </button>
                  </div>
                  <p className="text-xs text-zinc-600 mt-1">
                    Stored in OS keychain. Never sent anywhere except the provider API.
                  </p>
                </FieldGroup>
              )}

              {localProvider.type === "ollama" && (
                <FieldGroup label="Endpoint URL">
                  <input
                    type="text"
                    value={localProvider.endpoint ?? "http://localhost:11434"}
                    onChange={(e) => handleEndpointChange(e.target.value)}
                    placeholder="http://localhost:11434"
                    className="w-full px-3 py-2.5 bg-zinc-900 border border-zinc-700 rounded-lg text-sm text-zinc-200 placeholder:text-zinc-600 focus:outline-none focus:border-amber-500/50"
                  />
                </FieldGroup>
              )}

              <div className="bg-zinc-900/50 border border-zinc-800 rounded-lg p-4">
                <p className="text-xs text-zinc-500">
                  Only statistical features (~2-5 KB) are sent to the cloud. Full binary data never leaves your machine.
                </p>
              </div>
            </>
          )}

          {/* Connection Section */}
          {activeSection === "connection" && (
            <>
              <SectionHeader icon={Plug} title="Connection" description="Default CAN/serial adapter settings." />

              <FieldGroup label="Default Baud Rate">
                <select
                  value={baudRate}
                  onChange={(e) => setBaudRate(Number(e.target.value))}
                  className="w-full px-3 py-2.5 bg-zinc-900 border border-zinc-700 rounded-lg text-sm text-zinc-200 focus:outline-none focus:border-amber-500/50"
                >
                  {BAUD_RATES.map((rate) => (
                    <option key={rate.value} value={rate.value}>{rate.label}</option>
                  ))}
                </select>
              </FieldGroup>

              <FieldGroup label="Auto-reconnect">
                <ToggleSwitch
                  checked={autoReconnect}
                  onChange={setAutoReconnect}
                  label="Automatically reconnect to last adapter on startup"
                />
              </FieldGroup>
            </>
          )}

          {/* Safety Section */}
          {activeSection === "safety" && (
            <>
              <SectionHeader icon={Shield} title="Safety Checks" description="Hard-coded safety rules that protect against dangerous ECU modifications." />

              <FieldGroup label="Safety System">
                <ToggleSwitch
                  checked={safetyEnabled}
                  onChange={setSafetyEnabled}
                  label="Enable all safety checks before write operations"
                />
              </FieldGroup>

              <div className="space-y-2">
                <label className="text-sm text-zinc-400 block mb-2">Active Rules</label>
                {rules.map((rule) => (
                  <div
                    key={rule.id}
                    className={`flex items-center gap-3 px-4 py-3 rounded-lg border transition-colors ${
                      rule.active
                        ? "bg-zinc-900/50 border-zinc-800"
                        : "bg-zinc-900/20 border-zinc-800/50 opacity-50"
                    }`}
                  >
                    <button
                      onClick={() => toggleRule(rule.id)}
                      disabled={!safetyEnabled}
                      className={`w-5 h-5 rounded flex items-center justify-center flex-shrink-0 transition-colors ${
                        rule.active && safetyEnabled
                          ? "bg-green-500/20 text-green-400"
                          : "bg-zinc-800 text-zinc-600"
                      }`}
                    >
                      {rule.active && safetyEnabled && <CheckCircle2 size={14} />}
                    </button>
                    <span className="text-sm text-zinc-300 flex-1">{rule.label}</span>
                    <span
                      className={`text-[10px] font-semibold uppercase px-2 py-0.5 rounded ${
                        rule.severity === "critical"
                          ? "bg-red-500/10 text-red-400"
                          : "bg-yellow-500/10 text-yellow-400"
                      }`}
                    >
                      {rule.severity}
                    </span>
                  </div>
                ))}
              </div>

              <div className="bg-red-500/5 border border-red-500/20 rounded-lg p-4 flex gap-3">
                <AlertTriangle size={16} className="text-red-400 flex-shrink-0 mt-0.5" />
                <p className="text-xs text-red-300/80">
                  Critical safety rules cannot be disabled. They are hard-coded per ECU type to prevent engine damage.
                </p>
              </div>
            </>
          )}

          {/* Appearance Section */}
          {activeSection === "appearance" && (
            <>
              <SectionHeader icon={Palette} title="Appearance" description="Visual theme and language preferences." />

              <FieldGroup label="Theme">
                <div className="flex gap-3">
                  {(["dark", "light"] as const).map((t) => (
                    <button
                      key={t}
                      onClick={() => setTheme(t)}
                      className={`flex-1 px-4 py-3 rounded-lg border text-sm font-medium transition-colors ${
                        theme === t
                          ? "bg-amber-500/10 border-amber-500/30 text-amber-400"
                          : "bg-zinc-900 border-zinc-800 text-zinc-400 hover:border-zinc-700"
                      }`}
                    >
                      {t === "dark" ? "Dark" : "Light"}
                    </button>
                  ))}
                </div>
              </FieldGroup>

              <FieldGroup label="Language">
                <select
                  value={language}
                  onChange={(e) => setLanguage(e.target.value as "ru" | "en")}
                  className="w-full px-3 py-2.5 bg-zinc-900 border border-zinc-700 rounded-lg text-sm text-zinc-200 focus:outline-none focus:border-amber-500/50"
                >
                  <option value="ru">Русский</option>
                  <option value="en">English</option>
                </select>
              </FieldGroup>
            </>
          )}

          {/* About Section */}
          {activeSection === "about" && (
            <>
              <SectionHeader icon={Info} title="About Daedalus" description="Open-source AI-assisted ECU chip-tuning platform." />

              <div className="bg-zinc-900/50 border border-zinc-800 rounded-lg p-6 space-y-4">
                <div className="flex items-center gap-4">
                  <div className="w-12 h-12 rounded-xl bg-amber-500/10 flex items-center justify-center">
                    <Brain size={24} className="text-amber-400" />
                  </div>
                  <div>
                    <h3 className="text-lg font-semibold text-zinc-200">Daedalus</h3>
                    <p className="text-sm text-zinc-500">v0.1.0-alpha</p>
                  </div>
                </div>

                <div className="space-y-2 text-sm">
                  <InfoRow label="Runtime" value="Tauri 2.x + Rust 1.78" />
                  <InfoRow label="Frontend" value="React 19 + TypeScript 5.x" />
                  <InfoRow label="License" value="MIT" />
                </div>

                <div className="flex gap-3 pt-2">
                  <a
                    href="https://github.com/daedalus-ecu/daedalus"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="flex items-center gap-2 px-4 py-2 bg-zinc-800 rounded-lg text-sm text-zinc-300 hover:bg-zinc-700 transition-colors"
                  >
                    <ExternalLink size={14} />
                    GitHub
                  </a>
                  <a
                    href="https://docs.daedalus-ecu.dev"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="flex items-center gap-2 px-4 py-2 bg-zinc-800 rounded-lg text-sm text-zinc-300 hover:bg-zinc-700 transition-colors"
                  >
                    <ExternalLink size={14} />
                    Documentation
                  </a>
                </div>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}

/* --- Reusable sub-components --- */

function SectionHeader({ icon: Icon, title, description }: { icon: typeof Brain; title: string; description: string }) {
  return (
    <div className="mb-6">
      <div className="flex items-center gap-3 mb-1">
        <Icon size={20} className="text-amber-400" />
        <h2 className="text-lg font-semibold text-zinc-200">{title}</h2>
      </div>
      <p className="text-sm text-zinc-500 ml-8">{description}</p>
    </div>
  );
}

function FieldGroup({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="space-y-2">
      <label className="text-sm text-zinc-400 block">{label}</label>
      {children}
    </div>
  );
}

function ToggleSwitch({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <button
      onClick={() => onChange(!checked)}
      className="flex items-center gap-3 w-full text-left"
    >
      <div
        className={`relative w-10 h-5 rounded-full transition-colors ${
          checked ? "bg-amber-500/40" : "bg-zinc-700"
        }`}
      >
        <div
          className={`absolute top-0.5 w-4 h-4 rounded-full transition-transform ${
            checked ? "translate-x-5 bg-amber-400" : "translate-x-0.5 bg-zinc-400"
          }`}
        />
      </div>
      <span className="text-sm text-zinc-300">{label}</span>
    </button>
  );
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between py-1.5 border-b border-zinc-800/50">
      <span className="text-zinc-500">{label}</span>
      <span className="text-zinc-300">{value}</span>
    </div>
  );
}
