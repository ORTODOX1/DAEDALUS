import { invoke } from "@tauri-apps/api/core";
import type { AdapterInfo, DTCCode, J1939DTC, ChatMessage } from "../types";

// === Connection ===
export async function listAdapters(): Promise<AdapterInfo[]> {
  return invoke("list_adapters");
}

export async function connectAdapter(adapterId: string, baudRate: number): Promise<string> {
  return invoke("connect_adapter", { adapterId, baudRate });
}

export async function disconnectAdapter(): Promise<void> {
  return invoke("disconnect_adapter");
}

// === DTC ===
export async function readDTC(): Promise<DTCCode[]> {
  return invoke("read_dtc");
}

export async function readJ1939DTC(): Promise<J1939DTC[]> {
  return invoke("read_j1939_dtc");
}

export async function clearDTC(): Promise<void> {
  return invoke("clear_dtc");
}

// === AI ===
export async function aiChat(message: string): Promise<string> {
  return invoke("ai_chat", { message });
}

// === Reverse Engineering ===
export async function listSerialPorts(): Promise<string[]> {
  return invoke("list_serial_ports");
}

export async function connectMultimeter(port: string): Promise<boolean> {
  return invoke("connect_multimeter", { port });
}

export async function readMultimeter(): Promise<{ type: string; value: number; unit: string }> {
  return invoke("read_multimeter");
}

export async function loadPinoutDB(ecuType: string): Promise<unknown> {
  return invoke("load_pinout_db", { ecuType });
}

export async function analyzeReverse(ecuType: string, readings: unknown[]): Promise<unknown[]> {
  return invoke("analyze_reverse", { ecuType, readings });
}

// === System ===
export async function checkDrivers(): Promise<{ name: string; installed: boolean }[]> {
  return invoke("check_drivers");
}
