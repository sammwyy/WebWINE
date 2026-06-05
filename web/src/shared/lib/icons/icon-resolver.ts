import type { DirectoryEntry } from "@/core/wasm/worker";
import type { RuntimeBridge } from "@/core/bridge/runtime-bridge";

import { parseShortcutTarget, shortcutActionForName, shellActionForPath } from "../shortcut-target";
import { normaliseImageBytes, extractPeIcon } from "./icon-extractor";
import { ICON_REGISTRY, ICON_PLACEHOLDER, ICON_CANVAS_PX } from "./icon-registry";
import { AppRegistry } from "../app-registry";

export interface ResolvedIcon {
  src: string;
  overlay?: string;
}

export { ICON_PLACEHOLDER } from "./icon-registry";

const iconCache = new Map<string, Promise<ResolvedIcon>>();
const assetCache = new Map<string, Promise<string | null>>();

export function resolveIcon(
  entry: DirectoryEntry,
  runtime: RuntimeBridge,
): Promise<ResolvedIcon> {
  const key = entry.path;
  if (!iconCache.has(key)) {
    iconCache.set(key, doResolve(entry, runtime, 0));
  }
  return iconCache.get(key)!;
}

export function invalidateIconCache(): void {
  iconCache.clear();
  assetCache.clear();
}

async function doResolve(
  entry: DirectoryEntry,
  runtime: RuntimeBridge,
  depth: number,
): Promise<ResolvedIcon> {
  const name = entry.name.toLowerCase();

  const exactMatch = ICON_REGISTRY.paths[entry.path];
  if (exactMatch) {
    const src = await themeAsset(exactMatch);
    if (src) return { src };
  }

  if (/^[a-zA-Z]:\\$/.test(entry.path) || /^[a-zA-Z]:$/.test(entry.name)) {
    const src = await themeAsset("shell/drive_generic.webp");
    if (src) return { src };
  }

  if (entry.kind === "file" && name.endsWith(".lnk")) {
    return resolveLnk(entry, runtime);
  }

  if (entry.kind === "directory") {
    const src = (await themeAsset(ICON_REGISTRY.kinds.directory)) ?? ICON_PLACEHOLDER;
    return { src };
  }

  if (name.endsWith(".exe") || name.endsWith(".dll")) {
    const special = shellActionForPath(entry.path);
    if (special) {
      return { src: await shortcutActionIcon(special) };
    }
    const peIcon = await extractPeIcon(entry.path, runtime);
    const extMatch = ICON_REGISTRY.exts[name.split('.').pop()!] || ICON_REGISTRY.kinds.executable;
    const src = peIcon ?? (await themeAsset(extMatch)) ?? ICON_PLACEHOLDER;
    return { src };
  }

  const dot = entry.name.lastIndexOf(".");
  const ext = dot !== -1 ? entry.name.slice(dot + 1).toLowerCase() : "";
  const extAsset = ICON_REGISTRY.exts[ext] || `exts/${ext}.webp`;

  const src =
    (await themeAsset(extAsset).catch(() => null)) ??
    (await themeAsset(ICON_REGISTRY.kinds.default)) ??
    ICON_PLACEHOLDER;
  return { src };
}

async function resolveLnk(
  entry: DirectoryEntry,
  runtime: RuntimeBridge,
): Promise<ResolvedIcon> {
  const overlayPromise = themeAsset("shell/lnk.webp");

  try {
    const bytes = await runtime.readRawFile(entry.path);
    const target = parseShortcutTarget(new TextDecoder("utf-8", { fatal: false }).decode(bytes));

    if (target) {
      if (target.kind === "action") {
        const src = await shortcutActionIcon(target.action);
        const overlay = (await overlayPromise) ?? undefined;
        return { src, overlay };
      }

      const special = shellActionForPath(target.path);
      if (special) {
        const src = await shortcutActionIcon(special);
        const overlay = (await overlayPromise) ?? undefined;
        return { src, overlay };
      }

      const targetPath = target.path;
      const lastName = targetPath.replace(/[/\\]+$/, "").split(/[/\\]/).pop() ?? "";
      const synthetic: DirectoryEntry = {
        name: lastName || targetPath,
        path: targetPath,
        kind: lastName.includes(".") ? "file" : "directory",
        size: 0,
      };
      const base = await doResolve(synthetic, runtime, 1);
      const overlay = (await overlayPromise) ?? undefined;
      return { src: base.src, overlay };
    }
  } catch {
    const fallback = shortcutActionForName(entry.name);
    if (fallback) {
      const src = await shortcutActionIcon(fallback);
      const overlay = (await overlayPromise) ?? undefined;
      return { src, overlay };
    }
  }

  const fallback = shortcutActionForName(entry.name);
  if (fallback) {
    const src = await shortcutActionIcon(fallback);
    const overlay = (await overlayPromise) ?? undefined;
    return { src, overlay };
  }

  const src =
    (await themeAsset("exts/default.webp")) ?? ICON_PLACEHOLDER;
  const overlay = (await overlayPromise) ?? undefined;
  return { src, overlay };
}

async function shortcutActionIcon(action: string): Promise<string> {
  const app = AppRegistry.getAppByAction(action);
  if (app) {
    return (await themeAsset(app.icon)) ?? ICON_PLACEHOLDER;
  }

  switch (action) {
    case "this-pc":
      return (await themeAsset("places/thispc.webp")) ?? ICON_PLACEHOLDER;
    case "documents":
      return (await themeAsset("places/documents.webp")) ?? ICON_PLACEHOLDER;
    case "pictures":
      return (await themeAsset("places/pictures.webp")) ?? ICON_PLACEHOLDER;
    case "music":
      return (await themeAsset("places/music.webp")) ?? ICON_PLACEHOLDER;
    case "videos":
      return (await themeAsset("places/video.webp")) ?? ICON_PLACEHOLDER;
    default:
      return ICON_PLACEHOLDER;
  }
}

function themeAsset(relPath: string): Promise<string | null> {
  const url = `/theme/icons/${relPath}`;
  if (!assetCache.has(url)) {
    assetCache.set(url, fetchAndNormalise(url));
  }
  return assetCache.get(url)!;
}

async function fetchAndNormalise(url: string): Promise<string | null> {
  try {
    const resp = await fetch(url);
    if (!resp.ok) return null;
    const raw = new Uint8Array(await resp.arrayBuffer());
    return normaliseImageBytes(raw, ICON_CANVAS_PX);
  } catch {
    return null;
  }
}
