export type ShellAction =
  | "this-pc"
  | "documents"
  | "pictures"
  | "music"
  | "videos"
  | "explorer"
  | "upload-file"
  | "upload-folder";

export interface ShellActionDetail {
  action: "upload-file" | "upload-folder";
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
  return [
    "this-pc",
    "documents",
    "pictures",
    "music",
    "videos",
    "explorer",
    "upload-file",
    "upload-folder",
  ].includes(action);
}

export function shellActionForPath(path: string): ShellAction | null {
  const base = path.split("\\").pop()?.toLowerCase() ?? "";
  switch (base) {
    case "explorer.exe":
      return "explorer";
    case "uploadfile.exe":
      return "upload-file";
    case "uploadfolder.exe":
      return "upload-folder";
    default:
      return null;
  }
}

export function isShellActionPath(path: string): boolean {
  return shellActionForPath(path) !== null;
}

export function shortcutActionForName(name: string): ShellAction | null {
  const base = name.toLowerCase().replace(/\.lnk$/, "");
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
    case "file explorer":
      return "explorer";
    case "upload file":
      return "upload-file";
    case "upload folder":
      return "upload-folder";
    default:
      return null;
  }
}
