import type { ThemeDefinition } from "@/shared/lib/theme-registry";
import { getWallpaper, wallpaperThumbUrl } from "@/shared/lib/personalization";

/** Miniature desktop + window drawn from a theme's `preview` colours and preferred wallpaper. */
export function ThemeThumbnail({ theme }: { theme: ThemeDefinition }) {
  const p = theme.preview;
  const wallpaper = getWallpaper(theme.preferredWallpaper);
  // The thumbnail sits on top of the preview gradient, which shows if the file is missing.
  const desktop = wallpaper ? `url("${wallpaperThumbUrl(wallpaper)}") center / cover, ${p.desktop}` : p.desktop;

  return (
    <div className="ww-theme-thumb" style={{ background: desktop }}>
      <div
        className="ww-theme-thumb-window"
        style={{ background: p.frame, border: `1px solid ${p.accent}`, boxShadow: "0 2px 8px rgba(0,0,0,0.35)" }}
      >
        <div className="ww-theme-thumb-title" style={{ color: p.titleText }}>
          {theme.name}
        </div>
        <div className="ww-theme-thumb-body" style={{ background: p.body }} />
      </div>
      <div className="ww-theme-thumb-taskbar" style={{ background: p.taskbar }} />
    </div>
  );
}
