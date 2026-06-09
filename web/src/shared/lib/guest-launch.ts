import { openExplorer } from "../../apps/explorer/ExplorerApp";
import { launchProcessHidden } from "../../apps/process-console/ProcessConsoleApp";
import { openTextReader } from "../../apps/text-reader/TextReaderApp";
import { openRegedit } from "../../apps/regedit/RegeditApp";
import { openPhotoViewer } from "../../apps/photo-viewer/PhotoViewerApp";
import { openMediaPlayer } from "../../apps/media-player/MediaPlayerApp";
import type { RuntimeBridge } from "@/core/bridge/runtime-bridge";
import {
  parseShortcutTarget,
  shortcutActionForName,
  shellActionForPath,
  type ShellAction,
  type ShellActionDetail,
  isShellActionPath,
} from "./shortcut-target";
import { AppRegistry } from "./app-registry";

export const SHELL_ACTION_EVENT = "webwine:shell-action";

const GUEST_HOME = "C:\\Users\\guest";

export function requestShellAction(action: ShellActionDetail["action"], target?: string): void {
  window.dispatchEvent(
    new CustomEvent<ShellActionDetail>(SHELL_ACTION_EVENT, {
      detail: { action, target },
    }),
  );
}

export async function launchGuestPath(path: string, runtime: RuntimeBridge): Promise<void> {
  const normalized = normalizePath(path);
  const lower = normalized.toLowerCase();

  if (!normalized || normalized === "your pc") {
    openExplorer("", runtime);
    return;
  }

  if (lower.endsWith(".lnk")) {
    try {
      const bytes = await runtime.readRawFile(normalized);
      const target = parseShortcutTarget(new TextDecoder("utf-8").decode(bytes));
      if (!target) {
        const fallback = shortcutActionForName(normalized.split("\\").pop() ?? "");
        if (fallback) await launchShellAction(fallback, runtime);
        return;
      }
      if (target.kind === "action") {
        await launchShellAction(target.action, runtime);
      } else {
        await launchGuestPath(target.path, runtime);
      }
      return;
    } catch {
      const fallback = shortcutActionForName(normalized.split("\\").pop() ?? "");
      if (fallback) {
        await launchShellAction(fallback, runtime);
      }
      return;
    }
  }

  if (isShellActionPath(lower)) {
    const action = shellActionForPath(lower);
    if (action) {
      await launchShellAction(action, runtime);
    }
    return;
  }

  if (isDirectoryLike(normalized)) {
    openExplorer(normalized, runtime);
    return;
  }

  if (lower.endsWith(".exe") || lower.endsWith(".dll")) {
    await launchProcessHidden(normalized, runtime);
    return;
  }

  // Handle extensions via AppRegistry
  const extMatch = lower.match(/\.([^.\\]+)$/);
  if (extMatch) {
    const ext = extMatch[1];
    const apps = AppRegistry.getAppsForExtension(ext);
    if (apps.length > 0) {
      const app = apps[0];
      if (isShellActionPath(app.exePath)) {
         const action = shellActionForPath(app.exePath);
         if (action) await launchShellAction(action, runtime, normalized);
         return;
      }
      // If it's a real binary, launch it with the path as an argument
      await runtime.launchProcessWithArgs(app.exePath, normalized);
      return;
    }
  }

  await openTextReader(normalized, runtime);
}

async function launchShellAction(action: ShellAction, runtime: RuntimeBridge, targetPath?: string): Promise<void> {
  switch (action) {
    case "this-pc":
      openExplorer("", runtime);
      return;
    case "documents":
      openExplorer(`${GUEST_HOME}\\Documents`, runtime);
      return;
    case "pictures":
      openExplorer(`${GUEST_HOME}\\Pictures`, runtime);
      return;
    case "music":
      openExplorer(`${GUEST_HOME}\\Music`, runtime);
      return;
    case "videos":
      openExplorer(`${GUEST_HOME}\\Videos`, runtime);
      return;
    case "explorer":
      openExplorer(targetPath ?? "", runtime);
      return;
    case "editor":
      await openTextReader(targetPath ?? "", runtime);
      return;
    case "photo-viewer":
      await openPhotoViewer(targetPath ?? "", runtime);
      return;
    case "media-player":
      await openMediaPlayer(targetPath ?? "", runtime);
      return;
    case "regedit":
      openRegedit(runtime);
      return;
    default:
      // Delegate any unhandled or custom virtualApp actions to the client
      requestShellAction(action as ShellActionDetail["action"], targetPath);
      return;
  }
}

function isDirectoryLike(path: string): boolean {
  const lower = path.toLowerCase();
  if (/^[a-z]:\\?$/.test(lower)) return true;
  if (lower.startsWith("c:\\users\\") || lower.startsWith("c:\\windows\\")) {
    return !/\.[^\\]+$/.test(lower.split("\\").pop() ?? "");
  }
  return !/\.[^\\]+$/.test(lower.split("\\").pop() ?? "");
}

function normalizePath(path: string): string {
  const trimmed = path.trim().replace(/\//g, "\\");
  if (!trimmed) return "";
  if (/^[a-z]:\\?$/i.test(trimmed)) {
    return `${trimmed[0].toUpperCase()}:\\`;
  }
  return trimmed.replace(/\\+$/g, "");
}
