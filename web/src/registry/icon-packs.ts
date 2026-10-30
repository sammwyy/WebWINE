/**
 * Icon pack registry.
 *
 * A pack maps to `public/icons/<pack id>/<icon id>.<ext>`, where icon ids are
 * paths like `apps/explorer`, `shell/folder`, `places/documents`, `exts/txt`
 * (see `ICON_IDS` for the full set the shell uses). Icons missing from a pack
 * fall back to `DEFAULT_ICON_PACK`.
 *
 * Themes pick one with `preferredIconPack: "<id>"` (src/themes/<id>/index.ts).
 */

export interface IconPackEntry {
  id: string;
  name: string;
  /** file extension of the icons (default "webp") */
  ext?: string;
}

export const DEFAULT_ICON_PACK = "win10";

export const ICON_PACKS: IconPackEntry[] = [{ id: "win10", name: "Windows 10" }];

/** Icon ids used by the shell — a complete pack provides all of them. */
export const ICON_IDS = [
  "apps/controlpanel",
  "apps/explorer",
  "apps/mediaplayer",
  "apps/notepad",
  "apps/photoviewer",
  "apps/regedit",
  "apps/taskmgr",
  "exts/default",
  "exts/dll",
  "exts/txt",
  "places/desktop",
  "places/documents",
  "places/music",
  "places/pictures",
  "places/recycle",
  "places/thispc",
  "places/video",
  "shell/default_executable",
  "shell/drive_generic",
  "shell/drive_main",
  "shell/folder",
  "shell/lnk",
  "shell/msg_error",
  "shell/msg_inform",
  "shell/msg_question",
  "shell/msg_warning",
] as const;
