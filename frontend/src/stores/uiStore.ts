import { create } from "zustand";
import type { Panel } from "../types";

export type EditorTab = "hex" | "map2d" | "diff" | "safety" | "whatif";

interface UIState {
  activePanel: Panel;
  sidebarOpen: boolean;
  settingsOpen: boolean;
  editorTab: EditorTab;
  setActivePanel: (panel: Panel) => void;
  toggleSidebar: () => void;
  toggleSettings: () => void;
  setEditorTab: (tab: EditorTab) => void;
}

export const useUIStore = create<UIState>((set) => ({
  activePanel: "connection",
  sidebarOpen: true,
  settingsOpen: false,
  editorTab: "hex",
  setActivePanel: (panel) => set({ activePanel: panel }),
  toggleSidebar: () => set((s) => ({ sidebarOpen: !s.sidebarOpen })),
  toggleSettings: () => set((s) => ({ settingsOpen: !s.settingsOpen })),
  setEditorTab: (tab) => set({ editorTab: tab }),
}));
