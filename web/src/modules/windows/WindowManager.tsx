/**
 * WindowLayer — the window composer: renders all open windows from the store.
 *
 * This div is absolutely positioned over the desktop (it never covers the
 * taskbar). Each window is an independent, memoised `AppWindow`; pointer
 * events only reach the windows themselves.
 */

import { memo } from "react";
import { useWindowStore } from "@/state/windowStore";
import { AppWindow } from "./AppWindow";

const MemoAppWindow = memo(AppWindow);

export function WindowLayer() {
  const windows = useWindowStore((s) => s.windows);
  const activeId = useWindowStore((s) => s.activeId);

  return (
    <div id="window-layer" className="ww-window-layer" aria-label="Open windows">
      {windows.map((record) => (
        <MemoAppWindow
          key={record.id}
          win={record}
          focused={record.id === activeId && !record.minimized}
        />
      ))}
    </div>
  );
}
