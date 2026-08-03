/**
 * WindowLayer — renders all open windows from the window store.
 *
 * This div is absolutely positioned over the desktop. Each window is an
 * independent WindowFrame component; pointer events only reach children.
 */

import { memo } from "react";
import { useWindowStore } from "@/state/windowStore";
import type { WindowRecord } from "@/state/windowStore";
import { WindowFrame } from "./WindowFrame";

const MemoWindowFrame = memo(WindowFrame, (prev, next) => {
  const a = prev.record;
  const b = next.record;
  return (
    a === b ||
    (a.id === b.id &&
      a.title === b.title &&
      a.icon === b.icon &&
      a.minimized === b.minimized &&
      a.maximized === b.maximized &&
      a.zIndex === b.zIndex &&
      a.hideTitlebar === b.hideTitlebar &&
      a.content === b.content &&
      a.style === b.style)
  );
});

export function WindowLayer() {
  const windows = useWindowStore((s) => s.windows);

  return (
    <div
      id="window-layer"
      className="absolute inset-0 pointer-events-none"
      aria-label="Open windows"
    >
      {windows.map((record: WindowRecord) => (
        <MemoWindowFrame key={record.id} record={record} />
      ))}
    </div>
  );
}
