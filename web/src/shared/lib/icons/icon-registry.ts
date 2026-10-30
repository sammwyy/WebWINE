/**
 * Shared constants and configuration for icon resolution.
 */

/** Transparent 1×1 GIF — placeholder while icons load asynchronously */
export const ICON_PLACEHOLDER =
  "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";

/** CSS size of the icon element (px) */
export const ICON_CSS_PX = 48;

/** Canvas resolution: 2× for crisp HiDPI rendering */
export const ICON_CANVAS_PX = ICON_CSS_PX * Math.min(Math.ceil(window.devicePixelRatio ?? 1), 3);

/** Guest user profile root (matches the VM's path conventions) */
export const GUEST_PROFILE = "C:\\Users\\guest";

/** Icon ids (see src/registry/icon-packs.ts) used for kinds, extensions and known paths. */
export const ICON_REGISTRY = {
  kinds: {
    executable: "shell/default_executable",
    directory: "shell/folder",
    default: "exts/default",
  },
  exts: {
    exe: "shell/default_executable",
    dll: "exts/dll",
    txt: "exts/txt",
  } as Record<string, string>,
  paths: {
    "C:\\": "shell/drive_main",
    [`${GUEST_PROFILE}\\Desktop`]: "places/desktop",
    [`${GUEST_PROFILE}\\Documents`]: "places/documents",
    [`${GUEST_PROFILE}\\Music`]: "places/music",
    [`${GUEST_PROFILE}\\Pictures`]: "places/pictures",
    [`${GUEST_PROFILE}\\Videos`]: "places/video",
    "C:\\$Recycle.Bin": "places/recycle",
  } as Record<string, string>,
};
