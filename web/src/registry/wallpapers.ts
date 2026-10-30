/**
 * Wallpaper registry.
 *
 * Each entry maps to `public/wallpapers/<id>.<ext>` plus an optional
 * `public/wallpapers/<id>_thumb.<ext>` miniature (used by the Control Panel;
 * falls back to the full image). Run `npm run thumbs` to (re)generate thumbs.
 *
 * Themes pick one with `preferredWallpaper: "<id>"` (src/themes/<id>/index.ts).
 */

export interface WallpaperEntry {
  id: string;
  name: string;
  /** file extension (default "webp") */
  ext?: string;
  /** thumbnail extension (default: same as `ext`) */
  thumbExt?: string;
}

export const WALLPAPERS: WallpaperEntry[] = [
  { id: "win10", name: "Windows 10" },
  { id: "win7", name: "Windows 7" },
  { id: "vista", name: "Windows Vista" },
  { id: "xp", name: "Windows XP" },
];
