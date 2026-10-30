/**
 * Sound pack registry.
 *
 * A pack maps to `public/sounds/<pack id>/<sound id>.<ext>`. Packs don't need
 * every sound: a missing file is simply silent.
 *
 * Themes pick one with `preferredSoundPack: "<id>"` (src/themes/<id>/index.ts).
 */

/** System events that can play a sound. */
export const SOUND_EVENTS = {
  startup: "Start WebWINE",
  default: "Default Beep",
  error: "Critical Stop",
  warning: "Exclamation",
  info: "Asterisk",
  question: "Question",
  notify: "Notification",
  open: "Open Program",
  close: "Close Program",
  minimize: "Minimize",
  maximize: "Maximize",
  restore: "Restore",
  menu: "Menu Popup",
  navigate: "Start Navigation",
} as const;

export type SoundId = keyof typeof SOUND_EVENTS;

export interface SoundPackEntry {
  id: string;
  name: string;
  /** file extension of the sounds (default "wav") */
  ext?: string;
  /** per-sound extension overrides, e.g. `{ startup: "mp3" }` */
  exts?: Partial<Record<SoundId, string>>;
}

/** Built-in pseudo-pack that plays nothing. */
export const NO_SOUNDS = "none";

export const SOUND_PACKS: SoundPackEntry[] = [
  { id: NO_SOUNDS, name: "No Sounds" },
  { id: "win10", name: "Windows 10" },
  { id: "win7", name: "Windows 7" },
  { id: "vista", name: "Windows Vista" },
  { id: "xp", name: "Windows XP" },
];
