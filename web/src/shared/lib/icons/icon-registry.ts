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

export const ICON_REGISTRY = {
  kinds: {
    executable: "shell/default_executable.webp",
    directory: "shell/folder.webp",
    default: "exts/default.webp",
  },
  exts: {
    exe: "shell/default_executable.webp",
    dll: "exts/dll.webp",
    txt: "exts/txt.webp",
  } as Record<string, string>,
  paths: {
    "C:\\": "shell/drive_main.webp",
    [`${GUEST_PROFILE}\\Desktop`]: "places/desktop.webp",
    [`${GUEST_PROFILE}\\Documents`]: "places/documents.webp",
    [`${GUEST_PROFILE}\\Music`]: "places/music.webp",
    [`${GUEST_PROFILE}\\Pictures`]: "places/pictures.webp",
    [`${GUEST_PROFILE}\\Videos`]: "places/video.webp",
    "C:\\$Recycle.Bin": "places/recycle.webp",
  } as Record<string, string>,
};
