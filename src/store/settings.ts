import { create } from "zustand";
import { ipc, type Settings } from "../lib/ipc";

type SettingsState = {
  settings: Settings | null;
  systemAccent: string | null;
  load: () => Promise<void>;
  set: <K extends keyof Settings>(key: K, value: Settings[K]) => Promise<Settings>;
  replace: (s: Settings) => void;
};

export const useSettings = create<SettingsState>((set) => ({
  settings: null,
  systemAccent: null,
  load: async () => {
    const [settings, systemAccent] = await Promise.all([ipc.getSettings(), ipc.systemAccent().catch(() => null)]);
    set({ settings, systemAccent });
  },
  set: async (key, value) => {
    const settings = await ipc.setSetting(key, value);
    set({ settings });
    return settings;
  },
  replace: (settings) => set({ settings }),
}));

export const accentOf = (s: Settings | null, system: string | null) =>
  (s?.useSystemAccent && system) || s?.accent || "#6366F1";
