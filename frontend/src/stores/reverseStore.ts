import { create } from "zustand";
import type { MultimeterReading, TestPad, REAgentMessage, PinoutEntry } from "../types";

interface ReverseState {
  multimeterConnected: boolean;
  multimeterPort: string;
  selectedECU: string | null;
  readings: MultimeterReading[];
  testPads: TestPad[];
  agentMessages: REAgentMessage[];
  knownPinouts: PinoutEntry[];
  analysisRunning: boolean;
  setMultimeterConnected: (connected: boolean) => void;
  setMultimeterPort: (port: string) => void;
  selectECU: (ecu: string) => void;
  addReading: (reading: MultimeterReading) => void;
  setTestPads: (pads: TestPad[]) => void;
  addAgentMessage: (msg: REAgentMessage) => void;
  clearAgentMessages: () => void;
  setKnownPinouts: (pinouts: PinoutEntry[]) => void;
  setAnalysisRunning: (running: boolean) => void;
}

export const useReverseStore = create<ReverseState>((set) => ({
  multimeterConnected: false,
  multimeterPort: "",
  selectedECU: null,
  readings: [],
  testPads: [],
  agentMessages: [],
  knownPinouts: [],
  analysisRunning: false,
  setMultimeterConnected: (connected) => set({ multimeterConnected: connected }),
  setMultimeterPort: (port) => set({ multimeterPort: port }),
  selectECU: (ecu) => set({ selectedECU: ecu }),
  addReading: (reading) => set((s) => ({ readings: [...s.readings, reading] })),
  setTestPads: (pads) => set({ testPads: pads }),
  addAgentMessage: (msg) => set((s) => ({ agentMessages: [...s.agentMessages, msg] })),
  clearAgentMessages: () => set({ agentMessages: [] }),
  setKnownPinouts: (pinouts) => set({ knownPinouts: pinouts }),
  setAnalysisRunning: (running) => set({ analysisRunning: running }),
}));
