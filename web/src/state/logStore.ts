/**
 * useLogStore — append-only log entries for the system log panel.
 *
 * Trace-level events are silently dropped (they only go to debug consoles).
 */

import { create } from "zustand";
import type { LogEvent } from "../core/wasm/worker";

export interface LogEntry extends LogEvent {
  /** Auto-incremented key for React list rendering. */
  key: number;
}

let keyCounter = 0;

interface LogStore {
  entries: LogEntry[];
  append: (events: LogEvent[]) => void;
  clear: () => void;
}

export const useLogStore = create<LogStore>((set) => ({
  entries: [],

  append: (events) => {
    const filtered = events.filter((e) => e.level !== "trace");
    if (filtered.length === 0) return;
    set((state) => ({
      entries: [
        ...state.entries,
        ...filtered.map((e) => ({ ...e, key: ++keyCounter })),
      ],
    }));
  },

  clear: () => set({ entries: [] }),
}));

/** Convenience wrapper matching the original log(target, message, level) API. */
export function log(
  target: string,
  message: string,
  level: LogEvent["level"] = "info",
): void {
  useLogStore.getState().append([{ level, target, message }]);
}
