/**
 * Theme registry.
 *
 * Each import registers a theme (src/themes/<id>/index.ts calls `registerTheme`).
 * The first one is the default theme on a fresh install. A theme can pick its
 * preferred wallpaper, sound pack and icon pack (see the other registries here).
 */
import "../themes/win10dark";
import "../themes/win10";
import "../themes/win7";
import "../themes/vista";
import "../themes/xp";
