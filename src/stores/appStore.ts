import { create } from "zustand";

import i18n from "@/i18n";
import { defaultSettings, ipc } from "@/lib/ipc";
import type { AppInfo, AppSettings } from "@/types";

interface AppStore {
  settings: AppSettings;
  appInfo: AppInfo | null;
  bootstrapped: boolean;
  bootstrap: () => Promise<void>;
  updateSettings: (patch: Partial<AppSettings>) => Promise<void>;
  resetSettings: () => Promise<void>;
}

export const useAppStore = create<AppStore>((set, get) => ({
  settings: defaultSettings,
  appInfo: null,
  bootstrapped: false,

  bootstrap: async () => {
    if (get().bootstrapped) return;

    const [settingsResult, infoResult] = await Promise.allSettled([
      ipc.getSettings(),
      ipc.getAppInfo(),
    ]);

    if (settingsResult.status === "fulfilled") {
      set({ settings: settingsResult.value });
      await i18n.changeLanguage(settingsResult.value.language);
    }

    if (infoResult.status === "fulfilled") {
      set({ appInfo: infoResult.value });
    }

    set({ bootstrapped: true });
  },

  updateSettings: async (patch) => {
    const previous = get().settings;
    const next: AppSettings = { ...previous, ...patch };

    // Optimistic update keeps the UI instant; the backend is the source of truth.
    set({ settings: next });
    if (patch.language) {
      await i18n.changeLanguage(patch.language);
    }

    try {
      const saved = await ipc.saveSettings(next);
      set({ settings: saved });
    } catch (error) {
      console.error("no se pudo guardar la configuración", error);
      set({ settings: previous });
      await i18n.changeLanguage(previous.language);
    }
  },

  resetSettings: async () => {
    await get().updateSettings(defaultSettings);
  },
}));
