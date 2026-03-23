// === Connection ===
export type ConnectionStatus = "disconnected" | "connecting" | "connected" | "error";

export interface AdapterInfo {
  id: string;
  name: string;
  type: "socketcan" | "slcan" | "usb" | "j2534" | "serial";
  port: string;
  available: boolean;
}

export interface ECUInfo {
  name: string;
  manufacturer: string;
  processor: string;
  hwVersion: string;
  swVersion: string;
  protocol: "bdm" | "jtag" | "dap" | "kline" | "can";
  vehicleType: "truck" | "car" | "bus" | "agriculture";
}

// === DTC ===
export interface DTCCode {
  code: string;
  name: string;
  description: string;
  category: string;
  severity: "info" | "warning" | "critical";
  status: "active" | "stored" | "pending";
}

export interface J1939DTC {
  spn: number;
  fmi: number;
  name: string;
  description: string;
  category: string;
  severity: "info" | "warning" | "critical";
  ecu: string[];
}

// === AI ===
export interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  timestamp: number;
}

export interface AIProvider {
  type: "claude" | "openai" | "gemini" | "ollama";
  model: string;
  apiKey?: string;
  endpoint?: string;
}

// === Reverse Engineering ===
export interface MultimeterReading {
  id: string;
  padId: string;
  type: "voltage" | "resistance" | "continuity";
  value: number;
  unit: string;
  timestamp: number;
}

export interface TestPad {
  id: string;
  signal: string | null;
  voltage: number | null;
  resistance: number | null;
  x: number;
  y: number;
  confidence: number;
}

export interface PinoutEntry {
  ecu: string;
  manufacturer: string;
  vehicles: string[];
  processor: string;
  protocol: string;
  testPads: TestPad[];
}

export interface REAgentMessage {
  agentRole: string;
  content: string;
  confidence: number;
  round: number;
  phase: "seed" | "generate" | "critique" | "vote";
}

// === Project ===
export interface ProjectFile {
  path: string;
  name: string;
  size: number;
  type: "binary" | "map" | "config";
}

// === UI ===
export type Panel =
  | "connection"
  | "dtc"
  | "live"
  | "flash"
  | "editor"
  | "ai"
  | "reverse"
  | "settings";
