import { create } from "zustand";
import type { Panel } from "../types";

interface UIState {
  activePanel: Panel;
  sidebarOpen: boolean;
  settingsOpen: boolean;
  setActivePanel: (panel: Panel) => void;
  toggleSidebar: () => void;
  toggleSettings: () => void;
}

export const useUIStore = create<UIState>((set) => ({
  activePanel: "connection",
  sidebarOpen: true,
  settingsOpen: false,
  setActivePanel: (panel) => set({ activePanel: panel }),
  toggleSidebar: () => set((s) => ({ sidebarOpen: !s.sidebarOpen })),
  toggleSettings: () => set((s) => ({ settingsOpen: !s.settingsOpen })),
}));
