/**
 * useClipboardStore — single-entry file clipboard for copy/cut/paste operations.
 *
 * Holds at most one file at a time (matching classic Windows clipboard behaviour).
 */

import { create } from "zustand";

export type ClipboardOp = "copy" | "cut";

export interface ClipboardEntry {
  path: string;
  name: string;
  op: ClipboardOp;
}

interface ClipboardStore {
  entry: ClipboardEntry | null;
  set: (path: string, name: string, op: ClipboardOp) => void;
  clear: () => void;
  has: () => boolean;
  isCut: () => boolean;
}

export const useClipboardStore = create<ClipboardStore>((set, get) => ({
  entry: null,

  set: (path, name, op) => set({ entry: { path, name, op } }),

  clear: () => set({ entry: null }),

  has: () => get().entry !== null,

  isCut: () => get().entry?.op === "cut",
}));
