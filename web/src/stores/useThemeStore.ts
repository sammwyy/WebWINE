/**
 * useThemeStore — global theme selection state.
 *
 * Handles persistence in localStorage, injecting the theme CSS link tag,
 * applying the wallpaper to the desktop element, and notifying the rest of
 * the app via a custom event so icon caches can be invalidated.
 */

import { create } from "zustand";
import { invalidateIconCache } from "../lib/icon-resolver.js";

import { THEMES, type ThemeId, type TaskbarIconMode } from "../lib/themes.js";

const STORAGE_KEY = "webwine_theme";
const THEME_LINK_ID = "webwine-theme-css";
const DEFAULT_THEME: ThemeId = "fluent";

function storedTheme(): ThemeId {
  const raw = localStorage.getItem(STORAGE_KEY);
  return isThemeId(raw) ? raw : DEFAULT_THEME;
}

function isThemeId(v: string | null): v is ThemeId {
  return v === "fluent" || v === "luna" || v === "classic";
}

function applyThemeToDom(id: ThemeId): void {
  document.documentElement.dataset.theme = id;

  let link = document.getElementById(THEME_LINK_ID) as HTMLLinkElement | null;
  if (!link) {
    link = document.createElement("link");
    link.id = THEME_LINK_ID;
    link.rel = "stylesheet";
    document.head.append(link);
  }
  link.href = `/themes/${id}/${id}.css`;
  applyWallpaper(id);
}

function applyWallpaper(id: ThemeId): void {
  const desktop = document.getElementById("desktop");
  if (!desktop) return;
  const url = `/themes/${id}/wallpaper.webp`;
  const img = new Image();
  img.onload = () => {
    desktop.style.backgroundImage = `url('${url}')`;
    desktop.style.backgroundSize = "cover";
    desktop.style.backgroundPosition = "center";
  };
  img.onerror = () => {
    desktop.style.backgroundImage = "";
    desktop.style.backgroundSize = "";
    desktop.style.backgroundPosition = "";
  };
  img.src = url;
}

interface ThemeStore {
  theme: ThemeId;
  userTaskbarIcon: TaskbarIconMode | null;
  setTheme: (id: ThemeId) => void;
  setUserTaskbarIcon: (mode: TaskbarIconMode | null) => void;
  getEffectiveTaskbarIconMode: () => TaskbarIconMode;
  getEffectiveStartMenuLayout: () => StartMenuLayout;
}

export const useThemeStore = create<ThemeStore>((set, get) => ({
  theme: storedTheme(),
  userTaskbarIcon: null, // this could also be read from localStorage later

  setTheme: (id) => {
    localStorage.setItem(STORAGE_KEY, id);
    invalidateIconCache();
    applyThemeToDom(id);
    window.dispatchEvent(new Event("webwine:theme-changed"));
    set({ theme: id });
  },

  setUserTaskbarIcon: (mode) => {
    set({ userTaskbarIcon: mode });
  },

  getEffectiveTaskbarIconMode: () => {
    const { theme, userTaskbarIcon } = get();
    if (userTaskbarIcon) return userTaskbarIcon;
    const t = THEMES.find((x) => x.id === theme);
    return t ? t.taskbarIcon : "full";
  },

  getEffectiveStartMenuLayout: () => {
    const { theme } = get();
    const t = THEMES.find((x) => x.id === theme);
    return t ? t.startMenuLayout : "classic";
  },
}));

/** Initialize the theme from storage on first load. */
export function initTheme(): void {
  applyThemeToDom(storedTheme());
}
