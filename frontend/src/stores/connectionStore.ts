import { create } from "zustand";
import type { ConnectionStatus, AdapterInfo, ECUInfo } from "../types";

interface ConnectionState {
  status: ConnectionStatus;
  adapters: AdapterInfo[];
  selectedAdapter: string | null;
  baudRate: number;
  ecuInfo: ECUInfo | null;
  error: string | null;
  setStatus: (status: ConnectionStatus) => void;
  setAdapters: (adapters: AdapterInfo[]) => void;
  selectAdapter: (id: string) => void;
  setBaudRate: (rate: number) => void;
  setECUInfo: (info: ECUInfo | null) => void;
  setError: (error: string | null) => void;
}

export const useConnectionStore = create<ConnectionState>((set) => ({
  status: "disconnected",
  adapters: [],
  selectedAdapter: null,
  baudRate: 500000,
  ecuInfo: null,
  error: null,
  setStatus: (status) => set({ status }),
  setAdapters: (adapters) => set({ adapters }),
  selectAdapter: (id) => set({ selectedAdapter: id }),
  setBaudRate: (rate) => set({ baudRate: rate }),
  setECUInfo: (info) => set({ ecuInfo: info }),
  setError: (error) => set({ error }),
}));
