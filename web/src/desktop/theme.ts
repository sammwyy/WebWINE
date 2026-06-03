import { createWindow } from "../windows/manager.js";
import { invalidateIconCache } from "./icon-resolver.js";

export const THEME_STORAGE_KEY = "webwine_theme";

export type ThemeId = "fluent" | "luna" | "classic";

export interface ThemeOption {
  id: ThemeId;
  name: string;
  description: string;
}

export const THEMES: ThemeOption[] = [
  {
    id: "fluent",
    name: "Fluent (Win10)",
    description: "Dark square shell with blue focus accents.",
  },
  {
    id: "luna",
    name: "Luna (WinXP)",
    description: "Bright blue taskbar, rounded chrome, and warm menu panels.",
  },
  {
    id: "classic",
    name: "Classic (Win98)",
    description: "Flat gray controls, beveled borders, and compact chrome.",
  },
];

const DEFAULT_THEME: ThemeId = "fluent";

export function getActiveTheme(): ThemeId {
  return getStoredTheme();
}

const THEME_LINK_ID = "webwine-theme-css";

export function initTheme() {
  applyTheme(getStoredTheme());
}

export function getStoredTheme(): ThemeId {
  const raw = localStorage.getItem(THEME_STORAGE_KEY);
  return isThemeId(raw) ? raw : DEFAULT_THEME;
}

export function setTheme(id: ThemeId) {
  localStorage.setItem(THEME_STORAGE_KEY, id);
  invalidateIconCache();
  applyTheme(id);
  window.dispatchEvent(new Event("webwine:theme-changed"));
}

export function openThemesApp() {
  const { body } = createWindow({
    title: "Themes",
    icon: "T",
    width: 420,
    height: 346,
    resizable: false,
  });

  body.classList.add("themes-window-body");

  const wrap = document.createElement("div");
  wrap.className = "themes-app";

  for (const theme of THEMES) {
    const option = document.createElement("label");
    option.className = "theme-option";

    const input = document.createElement("input");
    input.type = "radio";
    input.name = "webwine-theme";
    input.value = theme.id;
    input.checked = theme.id === getStoredTheme();

    const preview = document.createElement("span");
    preview.className = `theme-preview theme-preview-${theme.id}`;
    preview.setAttribute("aria-hidden", "true");

    const text = document.createElement("span");
    text.className = "theme-option-text";

    const name = document.createElement("span");
    name.className = "theme-option-name";
    name.textContent = theme.name;

    const description = document.createElement("span");
    description.className = "theme-option-description";
    description.textContent = theme.description;

    text.append(name, description);
    option.append(input, preview, text);

    input.addEventListener("change", () => {
      if (input.checked) setTheme(theme.id);
    });

    wrap.append(option);
  }

  body.append(wrap);
}

function applyTheme(id: ThemeId) {
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

function applyWallpaper(id: ThemeId) {
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

function isThemeId(value: string | null): value is ThemeId {
  return value === "fluent" || value === "luna" || value === "classic";
}
