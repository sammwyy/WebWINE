/**
 * WindowLayer — renders all open windows from the window store.
 *
 * This div is absolutely positioned over the desktop. Each window is an
 * independent WindowFrame component; pointer events only reach children.
 */

import { useWindowStore } from "../../stores/useWindowStore.js";
import { WindowFrame } from "./WindowFrame.js";

export function WindowLayer() {
  const windows = useWindowStore((s) => s.windows);

  return (
    <div id="window-layer" aria-label="Open windows">
      {windows.map((record) => (
        <WindowFrame key={record.id} record={record} />
      ))}
    </div>
  );
}
