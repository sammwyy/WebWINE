/**
 * useSettingsStore — user preferences edited from the Control Panel.
 * Every field is persisted on its own as `ww_config_<field>` (shared/lib/config).
 *
 * `wallpaperId` / `soundPack` / `iconPack` = null means "the current theme's preferred
 * one"; applying a theme resets them, so the theme's wallpaper, sounds and icons get picked
 * automatically while the user can still choose others afterwards.
 */

import { create } from "zustand";
import { getConfig, setConfig } from "@/shared/lib/config";
import { onThemeApplied } from "@/shared/lib/theme-registry";

export type WallpaperMode = "image" | "color";
export type WallpaperFit = "fill" | "fit" | "stretch" | "center" | "tile";

export interface Settings {
  wallpaperMode: WallpaperMode;
  /** registry id (src/registry/wallpapers.ts), `custom`, or null = theme's preferred */
  wallpaperId: string | null;
  /** data: URI of a user-picked picture (used when `wallpaperId === "custom"`) */
  wallpaperCustom: string;
  wallpaperFit: WallpaperFit;
  wallpaperColor: string;
  /** sound pack id (src/registry/sound-packs.ts), or null = theme's preferred */
  soundPack: string | null;
  soundVolume: number;
  /** icon pack id (src/registry/icon-packs.ts), or null = theme's preferred */
  iconPack: string | null;
  showLogPanel: boolean;
  clockSeconds: boolean;
  /** show window titles next to the icons on the taskbar (themes may still hide them) */
  taskbarLabels: boolean;
  showDesktopIcons: boolean;
}

const DEFAULTS: Settings = {
  wallpaperMode: "image",
  wallpaperId: null,
  wallpaperCustom: "",
  wallpaperFit: "fill",
  wallpaperColor: "#3a6ea5",
  soundPack: null,
  soundVolume: 0.6,
  iconPack: null,
  showLogPanel: true,
  clockSeconds: false,
  taskbarLabels: true,
  showDesktopIcons: true,
};

interface SettingsState extends Settings {
  set: (patch: Partial<Settings>) => void;
  reset: (keys?: (keyof Settings)[]) => void;
}

function load(): Settings {
  const out = { ...DEFAULTS };
  for (const key of Object.keys(DEFAULTS) as (keyof Settings)[]) {
    (out as Record<string, unknown>)[key] = getConfig(key, DEFAULTS[key]);
  }
  return out;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  ...load(),
  set: (patch) => {
    for (const [key, value] of Object.entries(patch)) setConfig(key, value);
    set(patch);
  },
  reset: (keys = Object.keys(DEFAULTS) as (keyof Settings)[]) => {
    const patch: Partial<Settings> = {};
    for (const key of keys) {
      (patch as Record<string, unknown>)[key] = DEFAULTS[key];
      setConfig(key, null);
    }
    set(patch);
  },
}));

onThemeApplied(() => {
  useSettingsStore.getState().set({ wallpaperMode: "image", wallpaperId: null, soundPack: null, iconPack: null });
});
