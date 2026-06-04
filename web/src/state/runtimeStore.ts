/**
 * useRuntimeStore — holds the single RuntimeBridge instance and its ready state.
 *
 * Initialized once from App.tsx; all components that need the runtime read it here.
 */

import { create } from "zustand";
import { RuntimeBridge } from "../core/bridge/runtime-bridge";
import { useLogStore } from "./logStore";

interface RuntimeStore {
  runtime: RuntimeBridge | null;
  ready: boolean;
  /** Create the bridge and wait for the WASM worker to signal readiness. */
  init: () => Promise<void>;
}

import { loadVfsSnapshot } from "../shared/lib/vfs-storage";

export const useRuntimeStore = create<RuntimeStore>((set) => ({
  runtime: null,
  ready: false,

  init: async () => {
    const rt = new RuntimeBridge();

    // Wire global log events from the worker into the log store.
    rt.onGlobalLog((events) => {
      useLogStore.getState().append(events);
    });

    await rt.ready();
    await loadVfsSnapshot(rt);

    rt.onVfsChanged(() => {
      saveVfsSnapshot(rt);
    });

    set({ runtime: rt, ready: true });
  },
}));
