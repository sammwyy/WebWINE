/**
 * icon-resolver.ts
 *
 * Resolves desktop-entry icons to normalised PNG data-URIs (48 CSS px,
 * rendered at 2× for HiDPI).  All icons pass through a canvas so the
 * calling code always gets a plain `src` string — no onerror chains.
 *
 * Resolution order
 * ────────────────
 *   directory (known place) → places/<key>.webp → shell/folder.webp → SVG
 *   directory (generic)     → shell/folder.webp → SVG
 *   .lnk shortcut          → read target path from file → resolve target
 *                             icon + shell/lnk.webp overlay
 *   .exe / .dll (PE)        → extract embedded RT_GROUP_ICON → decode
 *                             best image → shell/default_executable.webp → SVG
 *   other file              → exts/<ext>.webp → exts/default.webp → SVG
 *
 * Theme icon layout  (under /themes/<id>/icons/)
 * ────────────────────────────────────────────────
 *   exts/<ext>.webp          per-extension icon
 *   exts/default.webp        catch-all file icon
 *   shell/folder.webp
 *   shell/default_executable.webp
 *   shell/lnk.webp           overlay badge for shortcuts
 *   places/documents.webp
 *   places/music.webp
 *   places/pictures.webp
 *   places/recycle.webp
 *   places/thispc.webp
 *   places/video.webp
 */

import type { DirectoryEntry } from "./worker.js";
import type { RuntimeBridge } from "./runtime-bridge.js";
import { useThemeStore } from "../stores/useThemeStore.js";

// ─── Public API ──────────────────────────────────────────────────────────────

export interface ResolvedIcon {
  /** Normalised PNG data-URI, ready to set as <img>.src */
  src: string;
  /** Optional overlay (lnk badge), same format */
  overlay?: string;
}

/** Transparent 1×1 GIF — placeholder while icons load asynchronously */
export const ICON_PLACEHOLDER =
  "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";

/**
 * Resolve an icon for a desktop entry.
 * Results are cached per (theme, path) — call invalidateIconCache() on theme change.
 */
export function resolveIcon(
  entry: DirectoryEntry,
  runtime: RuntimeBridge,
): Promise<ResolvedIcon> {
  const theme = useThemeStore.getState().theme;
  const key = `${theme}::${entry.path}`;
  if (!iconCache.has(key)) {
    iconCache.set(key, doResolve(entry, runtime, theme, 0));
  }
  return iconCache.get(key)!;
}

/** Wipe all caches. Must be called before switching theme. */
export function invalidateIconCache(): void {
  iconCache.clear();
  assetCache.clear();
}

// ─── Constants ───────────────────────────────────────────────────────────────

/** CSS size of the icon element (px) */
const ICON_CSS_PX = 48;

/** Canvas resolution: 2× for crisp HiDPI rendering */
const ICON_CANVAS_PX = ICON_CSS_PX * Math.min(Math.ceil(window.devicePixelRatio ?? 1), 3);

/** Guest user profile root (matches the VM's path conventions) */
const GUEST_PROFILE = "C:\\Users\\guest";

/** Maps absolute directory paths to place icon keys */
const PLACE_MAP: Record<string, string> = {
  [`${GUEST_PROFILE}\\Documents`]: "documents",
  [`${GUEST_PROFILE}\\Music`]:     "music",
  [`${GUEST_PROFILE}\\Pictures`]:  "pictures",
  [`${GUEST_PROFILE}\\Videos`]:    "video",
  "C:\\$Recycle.Bin":              "recycle",
};

// Inline SVG fallbacks (last resort, never fail)
const SVG_FILE   = svgFallback("#546e7a", "📄");
const SVG_FOLDER = svgFallback("#f9a825", "📁");
const SVG_EXE    = svgFallback("#1565c0", "⚙");

function svgFallback(bg: string, emoji: string): string {
  return (
    "data:image/svg+xml," +
    encodeURIComponent(
      `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48">` +
      `<rect width="48" height="48" rx="5" fill="${bg}"/>` +
      `<text x="24" y="36" font-size="28" text-anchor="middle" fill="white">${emoji}</text>` +
      "</svg>",
    )
  );
}

// ─── Caches ──────────────────────────────────────────────────────────────────

/** Per (theme, path) resolved icon */
const iconCache = new Map<string, Promise<ResolvedIcon>>();

/** Per theme-asset-URL normalised data-URI (or null if 404/error) */
const assetCache = new Map<string, Promise<string | null>>();

// ─── Resolution ──────────────────────────────────────────────────────────────

async function doResolve(
  entry: DirectoryEntry,
  runtime: RuntimeBridge,
  theme: string,
  depth: number,
): Promise<ResolvedIcon> {
  const name = entry.name.toLowerCase();

  // ── Shortcut (.lnk) ────────────────────────────────────────────────────────
  if (entry.kind === "file" && name.endsWith(".lnk")) {
    return resolveLnk(entry, runtime, theme);
  }

  // ── Directory ──────────────────────────────────────────────────────────────
  if (entry.kind === "directory") {
    const placeKey = PLACE_MAP[entry.path];
    const src =
      (placeKey ? await themeAsset(theme, `places/${placeKey}.webp`) : null) ??
      (await themeAsset(theme, "shell/folder.webp")) ??
      SVG_FOLDER;
    return { src };
  }

  // ── PE executable (.exe / .dll) ────────────────────────────────────────────
  if (name.endsWith(".exe") || name.endsWith(".dll")) {
    const peIcon = await extractPeIcon(entry.path, runtime);
    const src =
      peIcon ??
      (await themeAsset(theme, "shell/default_executable.webp")) ??
      SVG_EXE;
    return { src };
  }

  // ── Generic file ───────────────────────────────────────────────────────────
  const dot = entry.name.lastIndexOf(".");
  const ext = dot !== -1 ? entry.name.slice(dot + 1).toLowerCase() : "";
  const src =
    (ext ? await themeAsset(theme, `exts/${ext}.webp`) : null) ??
    (await themeAsset(theme, "exts/default.webp")) ??
    SVG_FILE;
  return { src };
}

async function resolveLnk(
  entry: DirectoryEntry,
  runtime: RuntimeBridge,
  theme: string,
): Promise<ResolvedIcon> {
  // Fetch overlay badge in parallel with target resolution
  const overlayPromise = themeAsset(theme, "shell/lnk.webp");

  try {
    const bytes = await runtime.readFile(entry.path);
    const targetPath = new TextDecoder("utf-8", { fatal: false })
      .decode(bytes)
      .replace(/\r?\n.*$/s, "") // first line only
      .trim();

    if (targetPath) {
      // Infer kind from path: no dot in last segment → directory
      const lastName = targetPath.replace(/[/\\]+$/, "").split(/[/\\]/).pop() ?? "";
      const synthetic: DirectoryEntry = {
        name: lastName || targetPath,
        path: targetPath,
        kind: lastName.includes(".") ? "file" : "directory",
        size: 0,
      };
      // depth=1 to prevent lnk→lnk recursion
      const base = await doResolve(synthetic, runtime, theme, 1);
      const overlay = (await overlayPromise) ?? undefined;
      return { src: base.src, overlay };
    }
  } catch {
    // Unreadable lnk — fall through
  }

  const src =
    (await themeAsset(theme, "exts/default.webp")) ?? SVG_FILE;
  const overlay = (await overlayPromise) ?? undefined;
  return { src, overlay };
}

// ─── Theme asset loading ──────────────────────────────────────────────────────

function themeAsset(theme: string, relPath: string): Promise<string | null> {
  const url = `/themes/${theme}/icons/${relPath}`;
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

// ─── Image normalisation (any format → 48px PNG data-URI via canvas) ─────────

/**
 * Convert raw image bytes (ICO container, PNG, BMP…) to a normalised
 * `size`×`size` PNG data-URI.  Returns null on any failure.
 */
async function normaliseImageBytes(
  raw: Uint8Array,
  size: number,
): Promise<string | null> {
  if (raw.length < 4) return null;

  // ICO container: reserved=0x0000, type=0x0001 (LE)
  if (raw[0] === 0 && raw[1] === 0 && raw[2] === 1 && raw[3] === 0) {
    return decodeIco(raw, size);
  }

  // Anything else: let the browser handle it
  return blobToCanvas(new Blob([raw.buffer.slice(raw.byteOffset, raw.byteOffset + raw.byteLength) as any]), size);
}

// ─── ICO container decoder ───────────────────────────────────────────────────

async function decodeIco(raw: Uint8Array, size: number): Promise<string | null> {
  const v = new DataView(raw.buffer, raw.byteOffset, raw.byteLength);
  const count = v.getUint16(4, true);
  if (count === 0) return null;

  interface IcoEntry { w: number; h: number; len: number; offset: number; }
  const entries: IcoEntry[] = [];
  for (let i = 0; i < count; i++) {
    const b = 6 + i * 16;
    if (b + 16 > raw.length) break;
    entries.push({
      w:      raw[b] || 256,
      h:      raw[b + 1] || 256,
      len:    v.getUint32(b + 8, true),
      offset: v.getUint32(b + 12, true),
    });
  }
  if (entries.length === 0) return null;

  // Pick largest image ≥ target size; fallback to absolute largest
  entries.sort((a, b) => b.w * b.h - a.w * a.h);
  const best = entries.find((e) => e.w >= size && e.h >= size) ?? entries[0];
  if (!best || best.offset + best.len > raw.length) return null;

  const imgBytes = raw.slice(best.offset, best.offset + best.len);
  return decodeIcoImage(imgBytes, size);
}

async function decodeIcoImage(raw: Uint8Array, size: number): Promise<string | null> {
  // PNG-in-ICO (Vista+ style)
  if (raw[0] === 0x89 && raw[1] === 0x50 && raw[2] === 0x4e && raw[3] === 0x47) {
    return blobToCanvas(new Blob([raw.buffer.slice(raw.byteOffset, raw.byteOffset + raw.byteLength) as any], { type: "image/png" }), size);
  }

  // DIB (BITMAPINFOHEADER)
  if (raw.length < 40) return null;
  const v = new DataView(raw.buffer, raw.byteOffset, raw.byteLength);
  const biWidth  = Math.abs(v.getInt32(4, true));
  const biHeight = Math.abs(v.getInt32(8, true)); // doubled: XOR + AND mask
  const bitCount = v.getUint16(14, true);
  const actualH  = biHeight >> 1; // ÷2
  const actualW  = biWidth;

  if (actualW <= 0 || actualH <= 0) return null;

  if (bitCount === 32) {
    return decode32BitDib(raw, v, actualW, actualH, size);
  }

  // For 24-bit and below, reconstruct a BMP file so the browser can parse it.
  // Note: AND-mask transparency is lost here (rare for theme icons).
  return dibToBmpBlob(raw, v, actualW, actualH * 2, bitCount, size);
}

async function decode32BitDib(
  raw: Uint8Array,
  _v: DataView,
  w: number,
  h: number,
  size: number,
): Promise<string | null> {
  const pixStart = 40; // right after BITMAPINFOHEADER (32-bit: no color table)
  const rowBytes = w * 4;
  if (raw.length < pixStart + rowBytes * h) return null;

  const canvas = new OffscreenCanvas(w, h);
  const ctx = canvas.getContext("2d")!;
  const imageData = ctx.createImageData(w, h);
  const dst = imageData.data;

  for (let row = 0; row < h; row++) {
    const srcRow = pixStart + (h - 1 - row) * rowBytes; // DIB is bottom-to-top
    const dstRow = row * w * 4;
    for (let col = 0; col < w; col++) {
      const s = srcRow + col * 4;
      const d = dstRow + col * 4;
      // BGRA → RGBA
      dst[d]     = raw[s + 2];
      dst[d + 1] = raw[s + 1];
      dst[d + 2] = raw[s];
      dst[d + 3] = raw[s + 3];
    }
  }

  ctx.putImageData(imageData, 0, 0);

  const out = new OffscreenCanvas(size, size);
  out.getContext("2d")!.drawImage(canvas, 0, 0, size, size);
  const blob = await out.convertToBlob({ type: "image/png" });
  return blobToDataUri(blob);
}

async function dibToBmpBlob(
  dib: Uint8Array,
  v: DataView,
  w: number,
  fullH: number,
  bitCount: number,
  size: number,
): Promise<string | null> {
  // Palette size (bytes)
  const palColors = bitCount <= 8 ? (1 << bitCount) : 0;
  const palBytes  = palColors * 4;
  const pixOffset = 14 + 40 + palBytes; // BITMAPFILEHEADER + BITMAPINFOHEADER + palette
  const fileSize  = pixOffset + Math.ceil(w * bitCount / 32) * 4 * fullH;

  const bmp = new Uint8Array(fileSize);
  const bv  = new DataView(bmp.buffer);

  // BITMAPFILEHEADER
  bmp[0] = 0x42; bmp[1] = 0x4d; // "BM"
  bv.setUint32(2,  fileSize, true);
  bv.setUint32(10, pixOffset, true);

  // Copy DIB (BITMAPINFOHEADER + palette + pixel data)
  bmp.set(dib.slice(0, dib.length), 14);

  return blobToCanvas(new Blob([bmp.buffer], { type: "image/bmp" }), size);
}

// ─── Canvas helpers ───────────────────────────────────────────────────────────

async function blobToCanvas(blob: Blob, size: number): Promise<string | null> {
  try {
    const url = URL.createObjectURL(blob);
    const img = await loadImage(url);
    URL.revokeObjectURL(url);

    const canvas = new OffscreenCanvas(size, size);
    const ctx = canvas.getContext("2d")!;
    ctx.clearRect(0, 0, size, size);
    ctx.drawImage(img, 0, 0, size, size);

    const outBlob = await canvas.convertToBlob({ type: "image/png" });
    return blobToDataUri(outBlob);
  } catch {
    return null;
  }
}

function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error("img load failed"));
    img.src = src;
  });
}

function blobToDataUri(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload  = () => resolve(reader.result as string);
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(blob);
  });
}

// ─── PE icon extraction ───────────────────────────────────────────────────────

const RT_ICON       = 3;
const RT_GROUP_ICON = 14;

async function extractPeIcon(
  path: string,
  runtime: RuntimeBridge,
): Promise<string | null> {
  try {
    const raw = await runtime.readFile(path);
    return await parsePeIcon(raw);
  } catch {
    return null;
  }
}

async function parsePeIcon(raw: Uint8Array): Promise<string | null> {
  try {
    return await doParsePeIcon(raw);
  } catch {
    return null;
  }
}

async function doParsePeIcon(raw: Uint8Array): Promise<string | null> {
  const v = new DataView(raw.buffer, raw.byteOffset, raw.byteLength);

  // ── MZ + PE header ─────────────────────────────────────────────────────────
  if (raw.length < 64 || v.getUint16(0, true) !== 0x5a4d) return null;
  const peOff = v.getUint32(60, true);
  if (peOff + 24 > raw.length) return null;
  if (v.getUint32(peOff, true) !== 0x00004550) return null; // "PE\0\0"

  const numSec   = v.getUint16(peOff + 6, true);
  const optSize  = v.getUint16(peOff + 20, true);
  const secBase  = peOff + 24 + optSize;

  // ── Find .rsrc section ─────────────────────────────────────────────────────
  let rsrcVA = 0, rsrcFileOff = 0;
  for (let i = 0; i < numSec; i++) {
    const s = secBase + i * 40;
    if (s + 40 > raw.length) break;
    const name = new TextDecoder().decode(raw.slice(s, s + 8)).replace(/\0+$/, "");
    if (name === ".rsrc") {
      rsrcVA      = v.getUint32(s + 12, true); // VirtualAddress
      rsrcFileOff = v.getUint32(s + 20, true); // PointerToRawData
      break;
    }
  }
  if (!rsrcFileOff) return null;

  // Convert resource-section RVA → file offset
  const rva2file = (rva: number) => rva - rsrcVA + rsrcFileOff;

  // ── Resource directory helpers (offsets relative to section start) ──────────
  const R = rsrcFileOff;

  function dirEntryCount(secOff: number): number {
    const a = R + secOff;
    if (a + 16 > raw.length) return 0;
    return v.getUint16(a + 12, true) + v.getUint16(a + 14, true);
  }

  interface ResEntry {
    id: number;
    subdirOff: number | null; // section-relative, when high bit set
    leafOff:   number | null;
  }

  function readEntry(secOff: number, idx: number): ResEntry | null {
    const a = R + secOff + 16 + idx * 8;
    if (a + 8 > raw.length) return null;
    const nameId = v.getUint32(a, true);
    const val    = v.getUint32(a + 4, true);
    const sub    = (val & 0x80000000) !== 0;
    return {
      id:        nameId & 0x7fffffff,
      subdirOff: sub  ? (val & 0x7fffffff) : null,
      leafOff:   !sub ? val : null,
    };
  }

  function readDataEntry(leafOff: number): { fileOff: number; size: number } | null {
    const a = R + leafOff;
    if (a + 8 > raw.length) return null;
    return { fileOff: rva2file(v.getUint32(a, true)), size: v.getUint32(a + 4, true) };
  }

  // ── Level 1: locate RT_GROUP_ICON and RT_ICON ──────────────────────────────
  const l1n = dirEntryCount(0);
  let grpDirOff: number | null = null;
  let icoDirOff: number | null = null;

  for (let i = 0; i < l1n; i++) {
    const e = readEntry(0, i);
    if (!e || e.subdirOff === null) continue;
    if (e.id === RT_GROUP_ICON) grpDirOff = e.subdirOff;
    if (e.id === RT_ICON)       icoDirOff = e.subdirOff;
  }
  if (grpDirOff === null || icoDirOff === null) return null;

  // ── Level 2 → Level 3: RT_GROUP_ICON → first group → first language ────────
  if (dirEntryCount(grpDirOff) === 0) return null;
  const grpNameEntry = readEntry(grpDirOff, 0);
  if (!grpNameEntry || grpNameEntry.subdirOff === null) return null;

  if (dirEntryCount(grpNameEntry.subdirOff) === 0) return null;
  const grpLangEntry = readEntry(grpNameEntry.subdirOff, 0);
  if (!grpLangEntry || grpLangEntry.leafOff === null) return null;

  const grpData = readDataEntry(grpLangEntry.leafOff);
  if (!grpData || grpData.fileOff + 6 > raw.length) return null;

  // ── Parse GRPICONDIR ───────────────────────────────────────────────────────
  //  WORD reserved, WORD type, WORD count
  //  GRPICONDIRENTRY[count]: BYTE w, BYTE h, BYTE cc, BYTE res, WORD planes,
  //                          WORD bitCount, DWORD bytesInRes, WORD id
  const gb         = grpData.fileOff;
  const grpCount   = v.getUint16(gb + 4, true);
  if (grpCount === 0) return null;

  interface GrpIcon { w: number; h: number; id: number; size: number; }
  const grpIcons: GrpIcon[] = [];
  for (let i = 0; i < grpCount; i++) {
    const eb = gb + 6 + i * 14;
    if (eb + 14 > raw.length) break;
    grpIcons.push({
      w:    raw[eb]     || 256,
      h:    raw[eb + 1] || 256,
      size: v.getUint32(eb + 8, true),
      id:   v.getUint16(eb + 12, true),
    });
  }
  if (grpIcons.length === 0) return null;

  // Pick the best: largest that is ≥ CSS size, else absolute largest
  grpIcons.sort((a, b) => b.w * b.h - a.w * a.h);
  const best = grpIcons.find((e) => e.w >= ICON_CSS_PX) ?? grpIcons[0];

  // ── Level 2 → Level 3: RT_ICON → id=best.id → first language ─────────────
  const l2n = dirEntryCount(icoDirOff);
  let icoChildOff: number | null = null;
  for (let i = 0; i < l2n; i++) {
    const e = readEntry(icoDirOff, i);
    if (!e || e.subdirOff === null) continue;
    if (e.id === best.id) { icoChildOff = e.subdirOff; break; }
  }
  if (icoChildOff === null) return null;

  if (dirEntryCount(icoChildOff) === 0) return null;
  const icoLangEntry = readEntry(icoChildOff, 0);
  if (!icoLangEntry || icoLangEntry.leafOff === null) return null;

  const icoData = readDataEntry(icoLangEntry.leafOff);
  if (!icoData || icoData.fileOff + best.size > raw.length) return null;

  // ── Decode the icon image ──────────────────────────────────────────────────
  const iconBytes = raw.slice(icoData.fileOff, icoData.fileOff + best.size);
  return decodeIcoImage(iconBytes, ICON_CANVAS_PX);
}
