import { useSyncExternalStore } from "react";
import type { ComponentType } from "react";
import { getConfig, setConfig } from "./config";

export type CaptionGlyphKind = "minimize" | "maximize" | "restore" | "close";

/** Optional React overrides for parts that can't be expressed in CSS alone. */
export interface ThemeComponents {
  /** glyph drawn inside the caption buttons */
  CaptionGlyph?: ComponentType<{ kind: CaptionGlyphKind }>;
  /** content of the taskbar start button (orb, logo…) */
  StartButton?: ComponentType;
  /**
   * Replacements for themeable UI components (see `useThemed` in
   * shared/ui/themed.ts), keyed by component name, e.g. `NavButtons`.
   * They receive the same props as the default component.
   */
  ui?: Partial<Record<string, ComponentType<any>>>;
}

/** Colours used to draw the theme thumbnail (e.g. in Control Panel). */
export interface ThemePreview {
  /** any CSS background */
  desktop: string;
  frame: string;
  titleText: string;
  body: string;
  accent: string;
  taskbar: string;
}

/**
 * A theme = a stylesheet scoped to `[data-theme~="<id>"]` (imported by the theme
 * module) + this definition. Registering it makes it selectable.
 */
export interface ThemeDefinition {
  id: string;
  name: string;
  description?: string;
  /**
   * Extra tokens added to `<html data-theme>`, so several themes can share a
   * stylesheet family (e.g. `['aero']` → `data-theme="win7 aero"`).
   */
  inherits?: string[];
  /** size of the window icons on the taskbar (default 16) */
  taskbarIconSize?: number;
  /**
   * Start menu layout: `classic` = two columns (XP / Vista / 7),
   * `modern` = Windows 10 rail + app list + tiles (default).
   */
  startMenu?: "classic" | "modern";
  /** id of the wallpaper (src/registry/wallpapers.ts) selected when the theme is applied */
  preferredWallpaper?: string;
  /** id of the sound pack (src/registry/sound-packs.ts) selected when the theme is applied */
  preferredSoundPack?: string;
  /** id of the icon pack (src/registry/icon-packs.ts) selected when the theme is applied */
  preferredIconPack?: string;
  /**
   * `dark` for themes whose window surfaces are dark. Sets `<html data-scheme>`,
   * which switches the semantic `--ww-*` colour tokens used by app layouts.
   */
  colorScheme?: "light" | "dark";
  /**
   * Per-app look variants, e.g. `{ mediaPlayer: "wmp11" }`. Apps read them with
   * `useThemeVariant("<app>", "<default>")`.
   */
  variants?: Partial<Record<string, string>>;
  preview: ThemePreview;
  components?: ThemeComponents;
}

const CONFIG_KEY = "theme";

type Listener = () => void;

const themes = new Map<string, ThemeDefinition>();
let themesSnapshot: ThemeDefinition[] = [];
let current: ThemeDefinition | null = null;
const listeners = new Set<Listener>();
const appliedListeners = new Set<(theme: ThemeDefinition) => void>();

/** `?theme=<id>` in the URL wins over the saved theme (handy for links / demos). */
const storedId = new URLSearchParams(location.search).get("theme") ?? getConfig<string | null>(CONFIG_KEY, null);

function emit() {
  listeners.forEach((l) => l());
}

function apply(theme: ThemeDefinition) {
  current = theme;
  const root = document.documentElement;
  root.dataset.theme = [theme.id, ...(theme.inherits ?? [])].join(" ");
  root.dataset.scheme = theme.colorScheme ?? "light";
  root.style.colorScheme = theme.colorScheme ?? "light";
  emit();
}

/**
 * Register a theme. The first registered theme is the default until the one
 * saved in the config gets registered.
 */
export function registerTheme(def: ThemeDefinition): ThemeDefinition {
  themes.set(def.id, def);
  themesSnapshot = [...themes.values()];
  if (!current || def.id === storedId || def.id === current.id) apply(def);
  else emit();
  return def;
}

/**
 * Switch to a registered theme (user action) and remember it. Listeners added
 * with `onThemeApplied` run afterwards — e.g. to pick the theme's preferred
 * wallpaper and sound pack.
 */
export function setTheme(id: string) {
  const theme = themes.get(id);
  if (!theme) return;
  setConfig(CONFIG_KEY, id);
  apply(theme);
  appliedListeners.forEach((l) => l(theme));
}

/** Called whenever the user applies a theme (not on startup restore). */
export function onThemeApplied(listener: (theme: ThemeDefinition) => void) {
  appliedListeners.add(listener);
  return () => appliedListeners.delete(listener);
}

export function getThemes(): ThemeDefinition[] {
  return themesSnapshot;
}

export function getCurrentTheme(): ThemeDefinition | null {
  return current;
}

export function subscribeThemes(listener: Listener) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function useThemes(): ThemeDefinition[] {
  return useSyncExternalStore(subscribeThemes, getThemes);
}

export function useCurrentTheme(): ThemeDefinition | null {
  return useSyncExternalStore(subscribeThemes, getCurrentTheme);
}

/** The active theme's look variant for `app` (see `ThemeDefinition.variants`). */
export function useThemeVariant(app: string, fallback: string): string {
  return useCurrentTheme()?.variants?.[app] ?? fallback;
}
