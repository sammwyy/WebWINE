/**
 * PE / ICO icon extraction and normalisation.
 *
 * Goal: always surface the *best* icon for the desktop (sharp, true-colour,
 * preferably with alpha) — not merely the first resource entry Windows stores.
 */

import type { RuntimeBridge } from "@/core/bridge/runtime-bridge";
import { ICON_CANVAS_PX, ICON_CSS_PX } from "./icon-registry";

// ── Image normalisation ─────────────────────────────────────────────────────

/**
 * Convert raw image bytes (ICO container, PNG, BMP…) to a normalised
 * `size`×`size` PNG data-URI. Returns null on any failure.
 */
export async function normaliseImageBytes(
  raw: Uint8Array,
  size: number,
): Promise<string | null> {
  if (raw.length < 4) return null;

  // ICO container: reserved=0x0000, type=0x0001 (LE)
  if (raw[0] === 0 && raw[1] === 0 && raw[2] === 1 && raw[3] === 0) {
    return decodeIco(raw, size);
  }

  return blobToCanvas(
    new Blob([
      raw.buffer.slice(
        raw.byteOffset,
        raw.byteOffset + raw.byteLength,
      ) as ArrayBuffer,
    ]),
    size,
  );
}

// ── ICO container ───────────────────────────────────────────────────────────

interface IcoDirEntry {
  w: number;
  h: number;
  bpp: number;
  colors: number;
  len: number;
  offset: number;
  /** PNG payload (Vista+) */
  isPng: boolean;
}

async function decodeIco(raw: Uint8Array, size: number): Promise<string | null> {
  const entries = parseIcoDirectory(raw);
  if (entries.length === 0) return null;

  const ranked = rankIconCandidates(
    entries.map((e) => ({
      w: e.w,
      h: e.h,
      bpp: e.bpp,
      colors: e.colors,
      isPng: e.isPng,
      payload: () => raw.slice(e.offset, e.offset + e.len),
    })),
    size,
  );

  for (const c of ranked) {
    const bytes = c.payload();
    if (bytes.length === 0) continue;
    const uri = await decodeIcoImage(bytes, size);
    if (uri && (await iconLooksUsable(uri))) return uri;
  }
  return null;
}

function parseIcoDirectory(raw: Uint8Array): IcoDirEntry[] {
  const v = new DataView(raw.buffer, raw.byteOffset, raw.byteLength);
  if (raw.length < 6) return [];
  const count = v.getUint16(4, true);
  if (count === 0) return [];

  const entries: IcoDirEntry[] = [];
  for (let i = 0; i < count; i++) {
    const b = 6 + i * 16;
    if (b + 16 > raw.length) break;
    const len = v.getUint32(b + 8, true);
    const offset = v.getUint32(b + 12, true);
    if (offset + len > raw.length || len < 4) continue;

    const w = raw[b] || 256;
    const h = raw[b + 1] || 256;
    const colors = raw[b + 2];
    // Planes / BitCount in the directory are often 0 for PNG; inspect payload.
    let bpp = v.getUint16(b + 6, true);
    const slice = raw.subarray(offset, offset + Math.min(len, 40));
    const isPng =
      slice[0] === 0x89 &&
      slice[1] === 0x50 &&
      slice[2] === 0x4e &&
      slice[3] === 0x47;
    if (isPng) {
      bpp = 32;
    } else if (bpp === 0 && slice.length >= 16) {
      // BITMAPINFOHEADER.biBitCount
      bpp = new DataView(
        slice.buffer,
        slice.byteOffset,
        slice.byteLength,
      ).getUint16(14, true);
    }

    entries.push({ w, h, bpp: bpp || 0, colors, len, offset, isPng });
  }
  return entries;
}

// ── Candidate ranking ───────────────────────────────────────────────────────

interface IconCandidate {
  w: number;
  h: number;
  bpp: number;
  colors: number;
  isPng: boolean;
  /** Lazy payload reader (so we don't copy every image up front). */
  payload: () => Uint8Array;
  /** Optional: which group this came from (lower = preferred by Windows). */
  groupIndex?: number;
  /** Resource id of the group (1 is common for the main app icon). */
  groupId?: number;
}

/**
 * Score an icon candidate for desktop display at `targetPx`.
 * Higher is better. Prefer true-colour + alpha, size near the target (or larger
 * for clean downscale), PNG-in-ICO, and the primary icon group.
 */
function scoreCandidate(c: IconCandidate, targetPx: number): number {
  const dim = Math.max(c.w, c.h);
  let score = 0;

  // Bit depth — the biggest visual quality lever.
  if (c.isPng || c.bpp >= 32) score += 10_000;
  else if (c.bpp >= 24) score += 6_000;
  else if (c.bpp >= 8) score += 2_000;
  else if (c.bpp >= 4) score += 500;
  else if (c.bpp > 0) score += 100;
  // ColorCount byte (0 = 256+ or true colour for modern entries).
  if (c.colors === 0 && c.bpp >= 8) score += 200;

  // Size: prefer >= target (downscale is sharp), penalise upscale heavily.
  if (dim >= targetPx) {
    // Prefer closest larger size, with a soft preference for 256/128/48.
    score += 3_000 - Math.min(2_500, (dim - targetPx) * 2);
    if (dim === 256 || dim === 128 || dim === 64 || dim === 48) score += 150;
  } else {
    // Upscaling looks bad — still usable if nothing else exists.
    score += dim * 4;
    score -= (targetPx - dim) * 20;
  }

  // Prefer earlier groups slightly (Windows' default ExtractIcon index 0),
  // but never enough to beat a better bpp/size from another group.
  if (c.groupIndex !== undefined) {
    score += Math.max(0, 80 - c.groupIndex * 15);
  }
  // Group id 1 is the conventional "main" application icon.
  if (c.groupId === 1) score += 40;

  return score;
}

function rankIconCandidates(
  candidates: IconCandidate[],
  targetPx: number,
): IconCandidate[] {
  return [...candidates].sort(
    (a, b) => scoreCandidate(b, targetPx) - scoreCandidate(a, targetPx),
  );
}

/**
 * Cheap sanity check: reject fully-transparent or tiny-nontransparent images
 * that often come from broken DIB/AND-mask decoding.
 */
async function iconLooksUsable(dataUri: string): Promise<boolean> {
  try {
    const img = await loadImage(dataUri);
    const w = Math.min(img.naturalWidth || img.width, 64);
    const h = Math.min(img.naturalHeight || img.height, 64);
    if (w < 1 || h < 1) return false;

    const canvas = new OffscreenCanvas(w, h);
    const ctx = canvas.getContext("2d")!;
    ctx.clearRect(0, 0, w, h);
    ctx.drawImage(img, 0, 0, w, h);
    const { data } = ctx.getImageData(0, 0, w, h);

    let opaque = 0;
    let nonZero = 0;
    for (let i = 0; i < data.length; i += 4) {
      const a = data[i + 3];
      if (a > 16) {
        opaque++;
        if (data[i] | data[i + 1] | data[i + 2]) nonZero++;
      }
    }
    const pixels = (data.length / 4) | 0;
    // At least ~2% visible pixels, and not a pure black rectangle with no detail
    // from a failed decode (allow pure-brand-black icons via nonZero check soft).
    if (opaque < Math.max(4, pixels * 0.02)) return false;
    if (nonZero < Math.max(2, opaque * 0.01) && opaque < pixels * 0.5) {
      // Almost all "opaque" pixels are black — still OK for some logos; accept.
    }
    return true;
  } catch {
    return false;
  }
}

// ── Single-image decode (PNG or DIB from ICO/PE) ────────────────────────────

async function decodeIcoImage(
  raw: Uint8Array,
  size: number,
): Promise<string | null> {
  // PNG-in-ICO (Vista+)
  if (
    raw.length >= 8 &&
    raw[0] === 0x89 &&
    raw[1] === 0x50 &&
    raw[2] === 0x4e &&
    raw[3] === 0x47
  ) {
    return blobToCanvas(
      new Blob(
        [
          raw.buffer.slice(
            raw.byteOffset,
            raw.byteOffset + raw.byteLength,
          ) as ArrayBuffer,
        ],
        { type: "image/png" },
      ),
      size,
    );
  }

  // DIB (BITMAPINFOHEADER)
  if (raw.length < 40) return null;
  const v = new DataView(raw.buffer, raw.byteOffset, raw.byteLength);
  const biSize = v.getUint32(0, true);
  if (biSize < 40) return null;

  const biWidth = Math.abs(v.getInt32(4, true));
  // Height is XOR + AND for icons (unless top-down negative, rare in icons).
  const biHeightRaw = v.getInt32(8, true);
  const biHeight = Math.abs(biHeightRaw);
  const bitCount = v.getUint16(14, true);
  const compression = v.getUint32(16, true); // 0 = BI_RGB, 3 = BI_BITFIELDS

  // Icon DIBs store XOR bitmap + AND mask stacked → height is 2× image height.
  // Some broken/odd resources store a single plane; detect via remaining size.
  let actualW = biWidth;
  let actualH = biHeight >> 1;
  if (actualH <= 0) actualH = biHeight;
  if (actualW <= 0 || actualH <= 0) return null;

  // If claiming 2× height doesn't fit the payload, treat height as single plane.
  const headerAndPal = estimateDibHeaderBytes(biSize, bitCount, compression);
  const rowBytes32 = ((actualW * bitCount + 31) >> 5) << 2;
  const xorBytes = rowBytes32 * actualH;
  if (raw.length < headerAndPal + xorBytes && biHeight === actualH * 2) {
    // try single-plane
    actualH = biHeight;
  }

  if (bitCount === 32) {
    return decode32BitDib(raw, biSize, compression, actualW, actualH, size);
  }
  if (bitCount === 24) {
    return decode24BitDib(raw, biSize, actualW, actualH, size);
  }
  if (bitCount === 8 || bitCount === 4 || bitCount === 1) {
    return decodePalettedDib(raw, biSize, bitCount, actualW, actualH, size);
  }

  // Fallback: wrap as BMP for the browser (loses AND-mask alpha).
  return dibToBmpBlob(raw, actualW, actualH * 2, bitCount, size);
}

function estimateDibHeaderBytes(
  biSize: number,
  bitCount: number,
  compression: number,
): number {
  let palColors = 0;
  if (bitCount <= 8) {
    palColors = 1 << bitCount;
  }
  let extra = 0;
  if (compression === 3 && (bitCount === 16 || bitCount === 32)) {
    extra = 12; // RGB masks
  }
  return biSize + palColors * 4 + extra;
}

async function decode32BitDib(
  raw: Uint8Array,
  biSize: number,
  compression: number,
  w: number,
  h: number,
  size: number,
): Promise<string | null> {
  let pixStart = biSize;
  if (compression === 3) pixStart += 12; // bitfields
  const rowBytes = w * 4;
  if (raw.length < pixStart + rowBytes * h) return null;

  // Detect whether alpha is actually used (many icons store 0 alpha = opaque).
  let maxA = 0;
  let anyA = false;
  for (let row = 0; row < h; row++) {
    const srcRow = pixStart + row * rowBytes;
    for (let col = 0; col < w; col++) {
      const a = raw[srcRow + col * 4 + 3];
      if (a > maxA) maxA = a;
      if (a > 0 && a < 255) anyA = true;
    }
  }
  const alphaIsGarbage = maxA === 0; // all zero → treat as opaque

  const canvas = new OffscreenCanvas(w, h);
  const ctx = canvas.getContext("2d")!;
  const imageData = ctx.createImageData(w, h);
  const dst = imageData.data;

  for (let row = 0; row < h; row++) {
    const srcRow = pixStart + (h - 1 - row) * rowBytes; // bottom-up
    const dstRow = row * w * 4;
    for (let col = 0; col < w; col++) {
      const s = srcRow + col * 4;
      const d = dstRow + col * 4;
      dst[d] = raw[s + 2];
      dst[d + 1] = raw[s + 1];
      dst[d + 2] = raw[s];
      dst[d + 3] = alphaIsGarbage ? 255 : raw[s + 3];
    }
  }

  // Apply 1-bpp AND mask when present (after XOR) and alpha wasn't useful.
  const andStart = pixStart + rowBytes * h;
  const andRowBytes = ((w + 31) >> 5) << 2;
  if (!anyA && raw.length >= andStart + andRowBytes * h) {
    for (let row = 0; row < h; row++) {
      const srcRow = andStart + (h - 1 - row) * andRowBytes;
      const dstRow = row * w * 4;
      for (let col = 0; col < w; col++) {
        const byte = raw[srcRow + (col >> 3)];
        const bit = (byte >> (7 - (col & 7))) & 1;
        if (bit) dst[dstRow + col * 4 + 3] = 0; // 1 = transparent
      }
    }
  }

  ctx.putImageData(imageData, 0, 0);
  return canvasToDataUri(canvas, size);
}

async function decode24BitDib(
  raw: Uint8Array,
  biSize: number,
  w: number,
  h: number,
  size: number,
): Promise<string | null> {
  const pixStart = biSize;
  const rowBytes = ((w * 3 + 3) >> 2) << 2;
  if (raw.length < pixStart + rowBytes * h) return null;

  const canvas = new OffscreenCanvas(w, h);
  const ctx = canvas.getContext("2d")!;
  const imageData = ctx.createImageData(w, h);
  const dst = imageData.data;

  for (let row = 0; row < h; row++) {
    const srcRow = pixStart + (h - 1 - row) * rowBytes;
    const dstRow = row * w * 4;
    for (let col = 0; col < w; col++) {
      const s = srcRow + col * 3;
      const d = dstRow + col * 4;
      dst[d] = raw[s + 2];
      dst[d + 1] = raw[s + 1];
      dst[d + 2] = raw[s];
      dst[d + 3] = 255;
    }
  }

  // AND mask
  const andStart = pixStart + rowBytes * h;
  const andRowBytes = ((w + 31) >> 5) << 2;
  if (raw.length >= andStart + andRowBytes * h) {
    for (let row = 0; row < h; row++) {
      const srcRow = andStart + (h - 1 - row) * andRowBytes;
      const dstRow = row * w * 4;
      for (let col = 0; col < w; col++) {
        const byte = raw[srcRow + (col >> 3)];
        const bit = (byte >> (7 - (col & 7))) & 1;
        if (bit) dst[dstRow + col * 4 + 3] = 0;
      }
    }
  }

  ctx.putImageData(imageData, 0, 0);
  return canvasToDataUri(canvas, size);
}

async function decodePalettedDib(
  raw: Uint8Array,
  biSize: number,
  bitCount: number,
  w: number,
  h: number,
  size: number,
): Promise<string | null> {
  const palColors = 1 << bitCount;
  const palStart = biSize;
  const palBytes = palColors * 4;
  const pixStart = palStart + palBytes;
  const rowBytes = ((w * bitCount + 31) >> 5) << 2;
  if (raw.length < pixStart + rowBytes * h) return null;

  const canvas = new OffscreenCanvas(w, h);
  const ctx = canvas.getContext("2d")!;
  const imageData = ctx.createImageData(w, h);
  const dst = imageData.data;

  for (let row = 0; row < h; row++) {
    const srcRow = pixStart + (h - 1 - row) * rowBytes;
    const dstRow = row * w * 4;
    for (let col = 0; col < w; col++) {
      let idx = 0;
      if (bitCount === 8) {
        idx = raw[srcRow + col];
      } else if (bitCount === 4) {
        const b = raw[srcRow + (col >> 1)];
        idx = col & 1 ? b & 0xf : b >> 4;
      } else {
        const b = raw[srcRow + (col >> 3)];
        idx = (b >> (7 - (col & 7))) & 1;
      }
      const p = palStart + idx * 4;
      const d = dstRow + col * 4;
      dst[d] = raw[p + 2];
      dst[d + 1] = raw[p + 1];
      dst[d + 2] = raw[p];
      dst[d + 3] = 255;
    }
  }

  const andStart = pixStart + rowBytes * h;
  const andRowBytes = ((w + 31) >> 5) << 2;
  if (raw.length >= andStart + andRowBytes * h) {
    for (let row = 0; row < h; row++) {
      const srcRow = andStart + (h - 1 - row) * andRowBytes;
      const dstRow = row * w * 4;
      for (let col = 0; col < w; col++) {
        const byte = raw[srcRow + (col >> 3)];
        const bit = (byte >> (7 - (col & 7))) & 1;
        if (bit) dst[dstRow + col * 4 + 3] = 0;
      }
    }
  }

  ctx.putImageData(imageData, 0, 0);
  return canvasToDataUri(canvas, size);
}

async function dibToBmpBlob(
  dib: Uint8Array,
  w: number,
  fullH: number,
  bitCount: number,
  size: number,
): Promise<string | null> {
  const palColors = bitCount <= 8 ? 1 << bitCount : 0;
  const palBytes = palColors * 4;
  const pixOffset = 14 + 40 + palBytes;
  const rowBytes = ((w * bitCount + 31) >> 5) << 2;
  const fileSize = pixOffset + rowBytes * fullH;

  const bmp = new Uint8Array(fileSize);
  const bv = new DataView(bmp.buffer);

  bmp[0] = 0x42;
  bmp[1] = 0x4d;
  bv.setUint32(2, fileSize, true);
  bv.setUint32(10, pixOffset, true);
  bmp.set(dib.subarray(0, Math.min(dib.length, fileSize - 14)), 14);

  return blobToCanvas(new Blob([bmp.buffer], { type: "image/bmp" }), size);
}

// ── Canvas helpers ──────────────────────────────────────────────────────────

async function canvasToDataUri(
  source: OffscreenCanvas,
  size: number,
): Promise<string | null> {
  const out = new OffscreenCanvas(size, size);
  const ctx = out.getContext("2d")!;
  ctx.imageSmoothingEnabled = true;
  ctx.imageSmoothingQuality = "high";
  ctx.clearRect(0, 0, size, size);
  // Contain-fit with high quality (don't stretch non-square oddly — icons are square).
  ctx.drawImage(source, 0, 0, size, size);
  const blob = await out.convertToBlob({ type: "image/png" });
  return blobToDataUri(blob);
}

async function blobToCanvas(blob: Blob, size: number): Promise<string | null> {
  try {
    const url = URL.createObjectURL(blob);
    const img = await loadImage(url);
    URL.revokeObjectURL(url);

    const canvas = new OffscreenCanvas(size, size);
    const ctx = canvas.getContext("2d")!;
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = "high";
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
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(blob);
  });
}

// ── PE icon extraction ──────────────────────────────────────────────────────

const RT_ICON = 3;
const RT_GROUP_ICON = 14;

/** Preferred language ids when several exist (English US, neutral, anything). */
const LANG_PREFERENCE = [0x0409, 0x0000, 0x0400];

export async function extractPeIcon(
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

  if (raw.length < 64 || v.getUint16(0, true) !== 0x5a4d) return null;
  const peOff = v.getUint32(60, true);
  if (peOff + 24 > raw.length) return null;
  if (v.getUint32(peOff, true) !== 0x00004550) return null;

  const numSec = v.getUint16(peOff + 6, true);
  const optSize = v.getUint16(peOff + 20, true);
  const magic = v.getUint16(peOff + 24, true); // PE32=0x10B, PE32+=0x20B
  const secBase = peOff + 24 + optSize;

  // Resource data directory (index 2) when available — more reliable than name.
  let rsrcDirRva = 0;
  let rsrcDirSize = 0;
  if (magic === 0x10b && peOff + 24 + 96 + 16 <= raw.length) {
    rsrcDirRva = v.getUint32(peOff + 24 + 96 + 16, true);
    rsrcDirSize = v.getUint32(peOff + 24 + 96 + 20, true);
  } else if (magic === 0x20b && peOff + 24 + 112 + 16 <= raw.length) {
    rsrcDirRva = v.getUint32(peOff + 24 + 112 + 16, true);
    rsrcDirSize = v.getUint32(peOff + 24 + 112 + 20, true);
  }

  // Map RVA → file offset via sections.
  const sections: { va: number; vs: number; rawPtr: number; rawSize: number }[] =
    [];
  for (let i = 0; i < numSec; i++) {
    const s = secBase + i * 40;
    if (s + 40 > raw.length) break;
    sections.push({
      va: v.getUint32(s + 12, true),
      vs: v.getUint32(s + 8, true),
      rawSize: v.getUint32(s + 16, true),
      rawPtr: v.getUint32(s + 20, true),
    });
  }

  const rva2file = (rva: number): number | null => {
    for (const sec of sections) {
      const span = Math.max(sec.vs, sec.rawSize);
      if (rva >= sec.va && rva < sec.va + span) {
        return sec.rawPtr + (rva - sec.va);
      }
    }
    return null;
  };

  // Locate resource section base.
  let rsrcVA = 0;
  let rsrcFileOff = 0;
  if (rsrcDirRva) {
    const fo = rva2file(rsrcDirRva);
    if (fo != null) {
      rsrcVA = rsrcDirRva;
      rsrcFileOff = fo;
    }
  }
  if (!rsrcFileOff) {
    for (let i = 0; i < numSec; i++) {
      const s = secBase + i * 40;
      if (s + 40 > raw.length) break;
      const name = new TextDecoder()
        .decode(raw.slice(s, s + 8))
        .replace(/\0+$/, "");
      if (name === ".rsrc") {
        rsrcVA = v.getUint32(s + 12, true);
        rsrcFileOff = v.getUint32(s + 20, true);
        break;
      }
    }
  }
  if (!rsrcFileOff) return null;

  const R = rsrcFileOff;
  const rva2fileRes = (rva: number) => {
    const f = rva2file(rva);
    return f ?? rva - rsrcVA + rsrcFileOff;
  };

  function dirEntryCount(secOff: number): { named: number; ids: number } {
    const a = R + secOff;
    if (a + 16 > raw.length) return { named: 0, ids: 0 };
    return {
      named: v.getUint16(a + 12, true),
      ids: v.getUint16(a + 14, true),
    };
  }

  interface ResEntry {
    id: number;
    isName: boolean;
    subdirOff: number | null;
    leafOff: number | null;
  }

  function readEntry(secOff: number, idx: number): ResEntry | null {
    const a = R + secOff + 16 + idx * 8;
    if (a + 8 > raw.length) return null;
    const nameId = v.getUint32(a, true);
    const val = v.getUint32(a + 4, true);
    const isDir = (val & 0x80000000) !== 0;
    return {
      id: nameId & 0x7fffffff,
      isName: (nameId & 0x80000000) !== 0,
      subdirOff: isDir ? val & 0x7fffffff : null,
      leafOff: !isDir ? val : null,
    };
  }

  function readDataEntry(
    leafOff: number,
  ): { fileOff: number; size: number } | null {
    const a = R + leafOff;
    if (a + 8 > raw.length) return null;
    const rva = v.getUint32(a, true);
    const size = v.getUint32(a + 4, true);
    const fileOff = rva2fileRes(rva);
    if (fileOff < 0 || fileOff + size > raw.length) return null;
    return { fileOff, size };
  }

  /** Pick preferred language leaf under a name/id directory. */
  function pickLanguageLeaf(langDirOff: number): number | null {
    const { named, ids } = dirEntryCount(langDirOff);
    const total = named + ids;
    if (total === 0) return null;

    const leaves: { id: number; leaf: number }[] = [];
    for (let i = 0; i < total; i++) {
      const e = readEntry(langDirOff, i);
      if (!e || e.leafOff === null) continue;
      leaves.push({ id: e.id, leaf: e.leafOff });
    }
    if (leaves.length === 0) return null;

    for (const pref of LANG_PREFERENCE) {
      const hit = leaves.find((l) => l.id === pref);
      if (hit) return hit.leaf;
    }
    return leaves[0].leaf;
  }

  // Level 1: RT_GROUP_ICON + RT_ICON
  const l1 = dirEntryCount(0);
  let grpDirOff: number | null = null;
  let icoDirOff: number | null = null;
  for (let i = 0; i < l1.named + l1.ids; i++) {
    const e = readEntry(0, i);
    if (!e || e.subdirOff === null || e.isName) continue;
    if (e.id === RT_GROUP_ICON) grpDirOff = e.subdirOff;
    if (e.id === RT_ICON) icoDirOff = e.subdirOff;
  }
  if (grpDirOff === null || icoDirOff === null) return null;

  // Index RT_ICON images by resource id → raw bytes
  const iconById = new Map<number, Uint8Array>();
  {
    const { named, ids } = dirEntryCount(icoDirOff);
    for (let i = 0; i < named + ids; i++) {
      const e = readEntry(icoDirOff, i);
      if (!e || e.subdirOff === null) continue;
      const leaf = pickLanguageLeaf(e.subdirOff);
      if (leaf === null) continue;
      const data = readDataEntry(leaf);
      if (!data) continue;
      iconById.set(
        e.id,
        raw.slice(data.fileOff, data.fileOff + data.size),
      );
    }
  }
  if (iconById.size === 0) return null;

  // Collect candidates from *all* icon groups (not just the first).
  const target = ICON_CANVAS_PX;
  const candidates: IconCandidate[] = [];

  {
    const { named, ids } = dirEntryCount(grpDirOff);
    let groupIndex = 0;
    for (let i = 0; i < named + ids; i++) {
      const e = readEntry(grpDirOff, i);
      if (!e || e.subdirOff === null) continue;
      const leaf = pickLanguageLeaf(e.subdirOff);
      if (leaf === null) continue;
      const grpData = readDataEntry(leaf);
      if (!grpData || grpData.fileOff + 6 > raw.length) continue;

      const gb = grpData.fileOff;
      const type = v.getUint16(gb + 2, true);
      if (type !== 1) continue; // not ICON
      const grpCount = v.getUint16(gb + 4, true);
      const groupId = e.isName ? 0 : e.id;

      for (let j = 0; j < grpCount; j++) {
        const eb = gb + 6 + j * 14;
        if (eb + 14 > raw.length) break;
        const w = raw[eb] || 256;
        const h = raw[eb + 1] || 256;
        const colors = raw[eb + 2];
        let bpp = v.getUint16(eb + 6, true);
        const bytesInRes = v.getUint32(eb + 8, true);
        const id = v.getUint16(eb + 12, true);
        const payload = iconById.get(id);
        if (!payload || payload.length === 0) continue;

        const isPng =
          payload[0] === 0x89 &&
          payload[1] === 0x50 &&
          payload[2] === 0x4e &&
          payload[3] === 0x47;
        if (isPng) bpp = 32;
        else if (bpp === 0 && payload.length >= 16) {
          bpp = new DataView(
            payload.buffer,
            payload.byteOffset,
            payload.byteLength,
          ).getUint16(14, true);
        }

        const gi = groupIndex;
        const gid = groupId;
        candidates.push({
          w,
          h,
          bpp: bpp || 0,
          colors,
          isPng,
          groupIndex: gi,
          groupId: gid,
          payload: () =>
            // Prefer declared size; clamp to available bytes.
            payload.slice(0, Math.min(bytesInRes || payload.length, payload.length)),
        });
      }
      groupIndex++;
    }
  }

  if (candidates.length === 0) return null;

  const ranked = rankIconCandidates(candidates, target);

  // Try top candidates until one decodes cleanly and looks usable.
  const tryN = Math.min(ranked.length, 12);
  for (let i = 0; i < tryN; i++) {
    const bytes = ranked[i].payload();
    if (bytes.length < 4) continue;
    const uri = await decodeIcoImage(bytes, ICON_CANVAS_PX);
    if (uri && (await iconLooksUsable(uri))) {
      return uri;
    }
  }

  // Last resort: first that decodes at all.
  for (const c of ranked) {
    const uri = await decodeIcoImage(c.payload(), ICON_CANVAS_PX);
    if (uri) return uri;
  }
  return null;
}
