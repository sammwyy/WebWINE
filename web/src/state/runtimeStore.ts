/**
 * useRuntimeStore — holds the single RuntimeBridge instance and its ready state.
 *
 * Initialized once from App.tsx; all components that need the runtime read it here.
 */

import { create } from "zustand";
import { RuntimeBridge } from "../core/bridge/runtime-bridge";
import { useLogStore } from "./logStore";
import { AppRegistry } from "../shared/lib/app-registry";

interface RuntimeStore {
  runtime: RuntimeBridge | null;
  ready: boolean;
  /** Create the bridge and wait for the WASM worker to signal readiness. */
  init: () => Promise<void>;
}

import { loadVfsSnapshot, saveVfsSnapshot } from "../shared/lib/vfs-storage";

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

    // The client owns the virtual-app list; materialize each in the guest VFS
    // (placeholder exe marker + Start Menu shortcut) now that the runtime is up.
    // The core no longer seeds these — see WebWineVm::register_app.
    await Promise.all(
      AppRegistry.getAll().map((app) =>
        rt.registerApp(app).catch(() => {
          /* non-fatal: a single app failing to register shouldn't block boot */
        }),
      ),
    );

    // Pin Task Manager on the guest desktop (Start Menu already has all apps).
    try {
      const taskmgr = AppRegistry.getAppByAction("task-manager");
      if (taskmgr) {
        const lnk = new TextEncoder().encode(taskmgr.exePath);
        await rt.mountFile(
          "C:\\Users\\guest\\Desktop\\Task Manager.lnk",
          lnk.buffer.slice(lnk.byteOffset, lnk.byteOffset + lnk.byteLength),
        );
      }
    } catch {
      /* non-fatal */
    }

    rt.onVfsChanged(() => {
      saveVfsSnapshot(rt);
    });

    set({ runtime: rt, ready: true });
  },
}));
