import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { AIProvider } from "../types";

interface SettingsState {
  aiProvider: AIProvider;
  theme: "dark" | "light";
  language: "ru" | "en";
  setAIProvider: (provider: AIProvider) => void;
  setTheme: (theme: "dark" | "light") => void;
  setLanguage: (lang: "ru" | "en") => void;
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      aiProvider: {
        type: "claude",
        model: "claude-sonnet-4-20250514",
      },
      theme: "dark",
      language: "ru",
      setAIProvider: (provider) => set({ aiProvider: provider }),
      setTheme: (theme) => set({ theme }),
      setLanguage: (lang) => set({ language: lang }),
    }),
    { name: "daedalus-settings" },
  ),
);
