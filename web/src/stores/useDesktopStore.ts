/**
 * useDesktopStore — desktop icon entries and their saved grid positions.
 *
 * Positions are persisted in localStorage so icons survive page reloads.
 */

import { create } from "zustand";
import type { DirectoryEntry } from "../lib/worker.js";
import type { RuntimeBridge } from "../lib/runtime-bridge.js";

export interface IconPosition {
  col: number;
  row: number;
}

const POSITIONS_KEY = "webwine.desktop.iconPositions";

function loadPositions(): Record<string, IconPosition> {
  try {
    const raw = localStorage.getItem(POSITIONS_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as Record<string, IconPosition>;
    return Object.fromEntries(
      Object.entries(parsed).filter(
        ([, p]) =>
          Number.isInteger(p.col) &&
          Number.isInteger(p.row) &&
          p.col >= 0 &&
          p.row >= 0,
      ),
    );
  } catch {
    return {};
  }
}

function savePositions(positions: Record<string, IconPosition>): void {
  try {
    localStorage.setItem(POSITIONS_KEY, JSON.stringify(positions));
  } catch {
    // Non-persistent storage should not break desktop rendering.
  }
}

interface DesktopStore {
  entries: DirectoryEntry[];
  positions: Record<string, IconPosition>;
  refreshing: boolean;

  refresh: (runtime: RuntimeBridge, desktopPath: string) => Promise<void>;
  setPosition: (path: string, pos: IconPosition) => void;
}

export const useDesktopStore = create<DesktopStore>((set, get) => ({
  entries: [],
  positions: loadPositions(),
  refreshing: false,

  refresh: async (runtime, desktopPath) => {
    set({ refreshing: true });
    try {
      const entries = await runtime.listDir(desktopPath);

      // Prune positions for entries that no longer exist.
      const activePaths = new Set(entries.map((e) => e.path));
      const positions = { ...get().positions };
      for (const p of Object.keys(positions)) {
        if (!activePaths.has(p)) delete positions[p];
      }
      savePositions(positions);

      set({ entries, positions, refreshing: false });
    } catch {
      set({ refreshing: false });
    }
  },

  setPosition: (path, pos) => {
    const positions = { ...get().positions, [path]: pos };
    savePositions(positions);
    set({ positions });
  },
}));
