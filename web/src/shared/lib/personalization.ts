/**
 * Resolves the user's personalization (wallpaper, icon pack, sound pack) from
 * the registries in src/registry, the settings store and the active theme.
 */

import { WALLPAPERS, type WallpaperEntry } from "@/registry/wallpapers";
import { DEFAULT_ICON_PACK, ICON_PACKS } from "@/registry/icon-packs";
import { NO_SOUNDS, SOUND_PACKS, type SoundId } from "@/registry/sound-packs";
import { useSettingsStore, type Settings } from "@/state/settingsStore";
import { getCurrentTheme, subscribeThemes, useCurrentTheme, type ThemeDefinition } from "./theme-registry";
import { invalidateIconCache } from "./icons/icon-resolver";

const BASE = import.meta.env.BASE_URL;

/* wallpapers */

export const wallpaperUrl = (w: WallpaperEntry) => `${BASE}wallpapers/${w.id}.${w.ext ?? "webp"}`;
export const wallpaperThumbUrl = (w: WallpaperEntry) =>
  `${BASE}wallpapers/${w.id}_thumb.${w.thumbExt ?? w.ext ?? "webp"}`;

export function getWallpaper(id: string | null | undefined): WallpaperEntry | undefined {
  return WALLPAPERS.find((w) => w.id === id);
}

/** Effective wallpaper id: the user's choice or the theme's preferred one. */
export function effectiveWallpaperId(settings: Pick<Settings, "wallpaperId">, theme: ThemeDefinition | null) {
  return settings.wallpaperId ?? theme?.preferredWallpaper ?? null;
}

/** CSS for the desktop wallpaper layer, or null to show the theme's own background. */
export function wallpaperStyle(settings: Settings, theme: ThemeDefinition | null): React.CSSProperties | null {
  if (settings.wallpaperMode === "color") return { background: settings.wallpaperColor };
  const id = effectiveWallpaperId(settings, theme);
  const entry = getWallpaper(id);
  const url = id === "custom" ? settings.wallpaperCustom : entry && wallpaperUrl(entry);
  if (!url) return null;
  const fit = {
    fill: { backgroundSize: "cover", backgroundPosition: "center" },
    fit: { backgroundSize: "contain", backgroundPosition: "center", backgroundRepeat: "no-repeat" },
    stretch: { backgroundSize: "100% 100%" },
    center: { backgroundSize: "auto", backgroundPosition: "center", backgroundRepeat: "no-repeat" },
    tile: { backgroundSize: "auto", backgroundRepeat: "repeat" },
  }[settings.wallpaperFit];
  return { backgroundImage: `url("${url}")`, backgroundRepeat: "no-repeat", ...fit };
}

/* icon packs */

export function effectiveIconPack(): string {
  const id = useSettingsStore.getState().iconPack ?? getCurrentTheme()?.preferredIconPack ?? DEFAULT_ICON_PACK;
  return ICON_PACKS.some((p) => p.id === id) ? id : DEFAULT_ICON_PACK;
}

/** URL of icon `id` (e.g. `apps/explorer`) in the active (or given) icon pack. */
export function iconUrl(id: string, pack = effectiveIconPack()): string {
  const ext = ICON_PACKS.find((p) => p.id === pack)?.ext ?? "webp";
  return `${BASE}icons/${pack}/${id}.${ext}`;
}

/** Same icon in the default pack — used when the active pack lacks it. */
export function iconFallbackUrl(id: string): string {
  return iconUrl(id, DEFAULT_ICON_PACK);
}

/** If `url` points into a non-default icon pack, the default pack's equivalent. */
export function iconUrlFallback(url: string): string | null {
  const m = url.match(/\/icons\/([^/]+)\/(.+)\.[^.]+$/);
  if (!m || m[1] === DEFAULT_ICON_PACK || !ICON_PACKS.some((p) => p.id === m[1])) return null;
  return iconFallbackUrl(m[2]);
}

/** Re-render on icon pack changes. Returns the active pack id. */
export function useIconPack(): string {
  const theme = useCurrentTheme();
  const chosen = useSettingsStore((s) => s.iconPack);
  return chosen ?? theme?.preferredIconPack ?? DEFAULT_ICON_PACK;
}

// When the effective pack changes, drop cached icons and let file views refresh.
let lastPack = effectiveIconPack();
const onPackMaybeChanged = () => {
  const pack = effectiveIconPack();
  if (pack === lastPack) return;
  lastPack = pack;
  invalidateIconCache();
  window.dispatchEvent(new Event("webwine:fs-changed"));
};
useSettingsStore.subscribe(onPackMaybeChanged);
subscribeThemes(onPackMaybeChanged);

/* sounds */

export function effectiveSoundPack(): string {
  return useSettingsStore.getState().soundPack ?? getCurrentTheme()?.preferredSoundPack ?? NO_SOUNDS;
}

export function soundUrl(pack: string, sound: SoundId): string | null {
  const entry = SOUND_PACKS.find((p) => p.id === pack);
  if (!entry || entry.id === NO_SOUNDS) return null;
  return `${BASE}sounds/${pack}/${sound}.${entry.exts?.[sound] ?? entry.ext ?? "wav"}`;
}

let pendingUnlock: (() => void) | null = null;

/** Play a system sound from `pack` (default: the active pack). Missing files are silent. */
export function playSound(sound: SoundId, pack = effectiveSoundPack()): void {
  const url = soundUrl(pack, sound);
  if (!url) return;
  const audio = new Audio(url);
  audio.volume = useSettingsStore.getState().soundVolume;
  audio.play().catch((err: DOMException) => {
    // Autoplay blocked (e.g. the startup sound): play on the first user gesture.
    if (err.name !== "NotAllowedError" || pendingUnlock) return;
    pendingUnlock = () => {
      pendingUnlock = null;
      void audio.play().catch(() => {});
    };
    window.addEventListener("pointerdown", pendingUnlock, { once: true });
  });
}
