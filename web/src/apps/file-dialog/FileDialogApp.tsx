/**
 * showFileDialog — backs the guest's GetOpenFileName / GetSaveFileName. It opens
 * the real Explorer component in "picker" mode (same layout, real icons, sidebar)
 * as a modal dialog, and resolves with the chosen guest path (or null on cancel),
 * which the caller posts back via `runtime.postDialogReply`.
 */

import { useWindowStore } from "@/state/windowStore";
import type { RuntimeBridge } from "@/core/bridge/runtime-bridge";
import type { UiEvent } from "@/core/wasm/worker";
import { ExplorerApp } from "../explorer/ExplorerApp";

type FileDialogEvent = Extract<UiEvent, { kind: "file_dialog" }>;

const DEFAULT_DIR = "C:\\Users\\guest\\Desktop";

export function showFileDialog(ev: FileDialogEvent, runtime: RuntimeBridge): Promise<string | null> {
  return new Promise((resolve) => {
    let winId = "";
    let settled = false;
    const finish = (path: string | null) => {
      if (settled) return;
      settled = true;
      if (winId) useWindowStore.getState().closeWindow(winId);
      resolve(path);
    };

    winId = useWindowStore.getState().openWindow({
      title: ev.title || (ev.save ? "Save As" : "Open"),
      icon: `${import.meta.env.BASE_URL}theme/icons/apps/explorer.webp`,
      variant: "dialog",
      width: 780,
      height: 520,
      content: <div />,
      onClose: () => finish(null),
    });

    useWindowStore.getState().setContent(
      winId,
      <ExplorerApp
        initialPath={ev.initial_dir || DEFAULT_DIR}
        runtime={runtime}
        windowId={winId}
        picker={{
          save: ev.save,
          filter: ev.filter,
          defaultName: ev.default_name,
          onPick: finish,
        }}
      />,
    );
  });
}
