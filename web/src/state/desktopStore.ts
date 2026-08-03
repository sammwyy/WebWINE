/**
 * useDesktopStore — desktop icon entries and their saved grid positions.
 *
 * Positions are persisted in localStorage so icons survive page reloads.
 * Icons snap to a discrete grid; new items claim the next free cell
 * (column-major: top→bottom, then next column) so uploads never stack on slot 0.
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

/** Default max rows per column when laying out new icons (Windows-like). */
export const DESKTOP_GRID_MAX_ROWS = 8;

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

function cellKey(col: number, row: number): string {
  return `${col},${row}`;
}

/** Set of occupied grid cells, optionally ignoring one or more paths. */
export function occupiedCells(
  positions: Record<string, IconPosition>,
  ignorePaths: Iterable<string> = [],
): Set<string> {
  const ignore = new Set(ignorePaths);
  const occupied = new Set<string>();
  for (const [path, pos] of Object.entries(positions)) {
    if (ignore.has(path)) continue;
    occupied.add(cellKey(pos.col, pos.row));
  }
  return occupied;
}

/**
 * Next free grid slot in column-major order (fill a column top→bottom, then
 * advance). Used for new uploads and for resolving drop collisions.
 */
export function findFreeSlot(
  occupied: Set<string>,
  maxRows = DESKTOP_GRID_MAX_ROWS,
  prefer?: IconPosition,
): IconPosition {
  if (prefer) {
    const key = cellKey(prefer.col, prefer.row);
    if (prefer.col >= 0 && prefer.row >= 0 && !occupied.has(key)) {
      return { col: prefer.col, row: prefer.row };
    }
  }

  for (let col = 0; col < 64; col++) {
    for (let row = 0; row < maxRows; row++) {
      if (!occupied.has(cellKey(col, row))) {
        return { col, row };
      }
    }
  }
  // Extremely full desktop: spill past maxRows in the last column.
  return { col: 0, row: maxRows };
}

/**
 * Claim `count` free slots, updating `occupied` as each is taken.
 * Starts scanning from optional prefer for the first slot only.
 */
export function findFreeSlots(
  occupied: Set<string>,
  count: number,
  maxRows = DESKTOP_GRID_MAX_ROWS,
  preferFirst?: IconPosition,
): IconPosition[] {
  const slots: IconPosition[] = [];
  for (let i = 0; i < count; i++) {
    const slot = findFreeSlot(
      occupied,
      maxRows,
      i === 0 ? preferFirst : undefined,
    );
    occupied.add(cellKey(slot.col, slot.row));
    slots.push(slot);
  }
  return slots;
}

/**
 * Snap a pixel point (relative to the icon grid content box) to a grid cell.
 */
export function pixelToGrid(
  x: number,
  y: number,
  cellWidth: number,
  cellHeight: number,
  pad = 12,
): IconPosition {
  const col = Math.max(0, Math.round((x - pad) / cellWidth));
  const row = Math.max(0, Math.round((y - pad) / cellHeight));
  return { col, row };
}

/**
 * Place `path` on the grid at `prefer` if free; otherwise the next free slot.
 * Does not collide with other icons (except when path already owns that cell).
 */
export function resolveDropPosition(
  positions: Record<string, IconPosition>,
  path: string,
  prefer: IconPosition,
  maxRows = DESKTOP_GRID_MAX_ROWS,
): IconPosition {
  const occupied = occupiedCells(positions, [path]);
  return findFreeSlot(occupied, maxRows, prefer);
}

/**
 * Ensure every entry has a unique grid position. Entries without a saved
 * position (or stacked on an already-taken cell) get the next free slot.
 */
export function ensureUniquePositions(
  entries: DirectoryEntry[],
  positions: Record<string, IconPosition>,
  maxRows = DESKTOP_GRID_MAX_ROWS,
): Record<string, IconPosition> {
  const next = { ...positions };
  const occupied = new Set<string>();

  // First pass: keep unique existing positions in entry order.
  const unplaced: string[] = [];
  for (const entry of entries) {
    const pos = next[entry.path];
    if (!pos) {
      unplaced.push(entry.path);
      continue;
    }
    const key = cellKey(pos.col, pos.row);
    if (occupied.has(key)) {
      // Collision — reassign later.
      unplaced.push(entry.path);
      continue;
    }
    occupied.add(key);
  }

  // Second pass: assign free slots to missing/colliding icons.
  for (const path of unplaced) {
    const slot = findFreeSlot(occupied, maxRows);
    occupied.add(cellKey(slot.col, slot.row));
    next[path] = slot;
  }

  return next;
}

interface DesktopStore {
  entries: DirectoryEntry[];
  positions: Record<string, IconPosition>;
  selectedIds: string[];
  refreshing: boolean;
  iconSize: DesktopIconSize;

  refresh: (runtime: RuntimeBridge, desktopPath: string) => Promise<void>;
  setPosition: (path: string, pos: IconPosition) => void;
  /** Place several icons at once (atomic persist). */
  setPositions: (updates: Record<string, IconPosition>) => void;
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

      // Prune positions for entries that no longer exist, then fill free slots
      // for anything new or stacked on the same cell.
      const activePaths = new Set(entries.map((e) => e.path));
      let positions = { ...get().positions };
      for (const p of Object.keys(positions)) {
        if (!activePaths.has(p)) delete positions[p];
      }
      positions = ensureUniquePositions(entries, positions);
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

  setPositions: (updates) => {
    const positions = { ...get().positions, ...updates };
    savePositions(positions);
    set({ positions });
  },

  selectIcon: (path, multi) => {
    set((state) => {
      if (multi) {
        return {
          selectedIds: state.selectedIds.includes(path)
            ? state.selectedIds.filter((id) => id !== path)
            : [...state.selectedIds, path],
        };
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
