/**
 * useClipboardStore — file clipboard for copy/cut/paste operations.
 *
 * Holds one or more filesystem nodes, matching multi-select shell behavior.
 */

import { create } from "zustand";

export type ClipboardOp = "copy" | "cut";

export interface ClipboardEntry {
  path: string;
  name: string;
  op: ClipboardOp;
}

interface ClipboardStore {
  entries: ClipboardEntry[];
  entry: ClipboardEntry | null;
  set: (path: string, name: string, op: ClipboardOp) => void;
  setMany: (entries: Omit<ClipboardEntry, "op">[], op: ClipboardOp) => void;
  clear: () => void;
  has: () => boolean;
  isCut: () => boolean;
}

export const useClipboardStore = create<ClipboardStore>((set, get) => ({
  entries: [],
  entry: null,

  set: (path, name, op) => {
    const entry = { path, name, op };
    set({ entry, entries: [entry] });
  },

  setMany: (entries, op) => {
    const next = entries.map((entry) => ({ ...entry, op }));
    set({ entry: next[0] ?? null, entries: next });
  },

  clear: () => set({ entry: null, entries: [] }),

  has: () => get().entries.length > 0,

  isCut: () => get().entries.some((entry) => entry.op === "cut"),
}));
