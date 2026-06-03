/**
 * useDesktopStore — desktop icon entries and their saved grid positions.
 *
 * Positions are persisted in localStorage so icons survive page reloads.
 */

import { create } from "zustand";
import type { DirectoryEntry } from "../core/wasm/worker";
import type { RuntimeBridge } from "../core/bridge/runtime-bridge";

export interface IconPosition {
  col: number;
  row: number;
}

export type DesktopIconSize = "small" | "medium" | "large";

export const DESKTOP_ICON_LAYOUTS: Record<
  DesktopIconSize,
  { iconSize: number; cellWidth: number; cellHeight: number }
> = {
  small: { iconSize: 52, cellWidth: 80, cellHeight: 92 },
  medium: { iconSize: 72, cellWidth: 96, cellHeight: 104 },
  large: { iconSize: 88, cellWidth: 112, cellHeight: 120 },
};

const POSITIONS_KEY = "webwine.desktop.iconPositions";
const ICON_SIZE_KEY = "webwine.desktop.iconSize";

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

function loadIconSize(): DesktopIconSize {
  try {
    const raw = localStorage.getItem(ICON_SIZE_KEY);
    if (raw === "small" || raw === "medium" || raw === "large") return raw;
  } catch {
    // Non-persistent storage should not break desktop rendering.
  }
  return "medium";
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
  selectedIds: string[];
  refreshing: boolean;
  iconSize: DesktopIconSize;

  refresh: (runtime: RuntimeBridge, desktopPath: string) => Promise<void>;
  setPosition: (path: string, pos: IconPosition) => void;
  selectIcon: (path: string, multi?: boolean) => void;
  clearSelection: () => void;
  setIconSize: (size: DesktopIconSize) => void;
}

export const useDesktopStore = create<DesktopStore>((set, get) => ({
  entries: [],
  positions: loadPositions(),
  selectedIds: [],
  refreshing: false,
  iconSize: loadIconSize(),

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

  selectIcon: (path, multi) => {
    set((state) => {
      if (multi) {
        return { selectedIds: state.selectedIds.includes(path) ? state.selectedIds : [...state.selectedIds, path] };
      }
      return { selectedIds: [path] };
    });
  },

  clearSelection: () => set({ selectedIds: [] }),

  setIconSize: (iconSize) => {
    try {
      localStorage.setItem(ICON_SIZE_KEY, iconSize);
    } catch {
      // Ignore persistence failures.
    }
    set({ iconSize });
  },
}));
