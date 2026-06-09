import { AppRegistry } from "./app-registry";

export type ShellAction =
  | "this-pc"
  | "documents"
  | "pictures"
  | "music"
  | "editor"
  | "photo-viewer"
  | "media-player"
  | string; // Allow any string for dynamic actions

export interface ShellActionDetail {
  action: "upload-file" | "upload-folder" | string;
  target?: string;
}

export type ShortcutTarget =
  | { kind: "path"; path: string }
  | { kind: "action"; action: ShellAction };

export function parseShortcutTarget(text: string): ShortcutTarget | null {
  const raw = text.split(/\r?\n/, 1)[0]?.trim() ?? "";
  if (!raw) return null;

  const actionMatch = raw.match(/^action:(.+)$/i);
  if (actionMatch) {
    const action = actionMatch[1].trim().toLowerCase();
    if (isShellAction(action)) {
      return { kind: "action", action };
    }
    return null;
  }

  return { kind: "path", path: raw };
}

export function isShellAction(action: string): action is ShellAction {
  if ([
    "this-pc",
    "documents",
    "pictures",
    "music",
    "editor",
    "photo-viewer",
    "media-player",
  ].includes(action)) return true;
  
  return AppRegistry.getAppByAction(action) !== undefined;
}

export function shellActionForPath(path: string): ShellAction | null {
  const app = AppRegistry.getAppByExe(path);
  if (app) return app.action;

  const base = path.split("\\").pop()?.toLowerCase() ?? "";
  switch (base) {
    default:
      return null;
  }
}

export function isShellActionPath(path: string): boolean {
  return shellActionForPath(path) !== null;
}

export function shortcutActionForName(name: string): ShellAction | null {
  const base = name.toLowerCase().replace(/\.lnk$/, "");
  const app = AppRegistry.getAppByName(base);
  if (app) return app.action;

  switch (base) {
    case "your pc":
      return "this-pc";
    case "documents":
      return "documents";
    case "pictures":
      return "pictures";
    case "music":
      return "music";
    case "videos":
      return "videos";
    default:
      return null;
  }
}
