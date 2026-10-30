/**
 * AppWindow — connects a WindowRecord from the store to the themed
 * `WindowFrame`: focus, dragging by the caption, edge resizing, maximize.
 */

import { useLayoutEffect, useMemo, useRef } from "react";
import type { CSSProperties, PointerEvent as ReactPointerEvent } from "react";
import { useWindowStore } from "@/state/windowStore";
import { useCurrentTheme } from "@/shared/lib/theme-registry";
import type { WindowRecord } from "@/state/windowStore";
import { WindowContext } from "./window-context";
import type { WindowHandle } from "./window-context";
import { WindowFrame } from "./WindowFrame";
import { WindowBody } from "./WindowBody";
import { ResizeHandles } from "./ResizeHandles";
import type { ResizeDirection } from "./ResizeHandles";

/** Track a pointer drag on `window` until release. */
export function trackPointer(
  e: { clientX: number; clientY: number },
  onMove: (dx: number, dy: number) => void,
  cursor?: string,
) {
  const startX = e.clientX;
  const startY = e.clientY;
  const prevCursor = document.body.style.cursor;
  if (cursor) document.body.style.cursor = cursor;
  document.body.style.userSelect = "none";
  // Guest canvases / iframes must not swallow the pointer while dragging.
  document.body.classList.add("ww-dragging");

  const move = (ev: PointerEvent) => onMove(ev.clientX - startX, ev.clientY - startY);
  const up = () => {
    document.body.style.cursor = prevCursor;
    document.body.style.userSelect = "";
    document.body.classList.remove("ww-dragging");
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up);
}

/** Start moving window `id` from a pointer event (used by custom title bars too). */
export function beginWindowDrag(id: string, e: ReactPointerEvent) {
  const store = useWindowStore.getState();
  const win = store.windows.find((w) => w.id === id);
  if (!win || e.button !== 0 || win.maximized) return;
  const layer = document.getElementById("window-layer");
  const maxY = (layer?.clientHeight ?? window.innerHeight) - 24;
  const { x, y } = win;
  trackPointer(e, (dx, dy) => {
    store.moveWindow(id, x + dx, Math.min(maxY, Math.max(0, y + dy)));
  });
}

interface AppWindowProps {
  win: WindowRecord;
  focused: boolean;
}

export function AppWindow({ win, focused }: AppWindowProps) {
  const ref = useRef<HTMLDivElement>(null);
  const store = useWindowStore.getState();
  const theme = useCurrentTheme();

  const handle = useMemo<WindowHandle>(
    () => ({
      id: win.id,
      close: () => useWindowStore.getState().closeWindow(win.id),
      focus: () => useWindowStore.getState().focusWindow(win.id),
      minimize: () => useWindowStore.getState().minimizeWindow(win.id),
      toggleMaximize: () => useWindowStore.getState().toggleMaximize(win.id),
      setTitle: (title) => useWindowStore.getState().setTitle(win.id, title),
    }),
    [win.id],
  );

  // Fit the outer size so the client area matches `clientSize` with this theme's frame.
  useLayoutEffect(() => {
    const el = ref.current;
    const client = el?.querySelector<HTMLElement>(":scope > .ww-window-client");
    if (!el || !client || !win.clientSize || win.maximized) return;
    const dw = el.offsetWidth - client.clientWidth;
    const dh = el.offsetHeight - client.clientHeight;
    const width = win.clientSize.width + dw;
    const height = win.clientSize.height + dh;
    if (width !== win.width || height !== win.height) {
      useWindowStore.getState().setBounds(win.id, { x: win.x, y: win.y, width, height });
    }
    // Only once per window / theme switch, not on every user resize.
  }, [win.id, theme]);

  const resizable = win.resizable && !win.hideTitlebar;
  const maximizable = win.maximizable ?? resizable;

  const onResizeStart = (dir: ResizeDirection, e: ReactPointerEvent) => {
    if (e.button !== 0) return;
    e.stopPropagation();
    store.focusWindow(win.id);
    const start = {
      x: win.x,
      y: win.y,
      width: win.width,
      height: win.height ?? ref.current?.offsetHeight ?? 200,
    };
    const min = store.getMinSize(win.id);
    const cursor = getComputedStyle(e.currentTarget).cursor;

    trackPointer(
      e,
      (dx, dy) => {
        let { x, y, width, height } = start;
        if (dir.includes("e")) width += dx;
        if (dir.includes("s")) height += dy;
        if (dir.includes("w")) {
          width = Math.max(min.width, width - dx);
          x = start.x + start.width - width;
        }
        if (dir.includes("n")) {
          height = Math.max(min.height, height - dy);
          y = Math.max(0, start.y + start.height - height);
          height = start.y + start.height - y;
        }
        useWindowStore.getState().setBounds(win.id, { x, y, width, height });
      },
      cursor,
    );
  };

  const style: CSSProperties = win.maximized
    ? { left: 0, top: 0, width: "100%", height: "100%", zIndex: win.zIndex }
    : {
        left: win.x,
        top: win.y,
        width: win.width,
        height: win.height ?? "auto",
        zIndex: win.zIndex,
      };
  if (win.minimized) style.display = "none";

  const surface = win.surface ?? (win.variant === "dialog" ? "face" : "white");

  return (
    <WindowFrame
      ref={ref}
      id={win.id}
      title={win.title}
      icon={win.icon}
      active={focused}
      maximized={win.maximized}
      dialog={win.variant === "dialog"}
      frameless={win.hideTitlebar}
      minimizable={win.minimizable ?? true}
      maximizable={maximizable}
      onMinimize={handle.minimize}
      onToggleMaximize={handle.toggleMaximize}
      onClose={handle.close}
      style={{ ...style, minWidth: win.minWidth, minHeight: win.minHeight }}
      onPointerDownCapture={handle.focus}
      onContextMenu={(e) => e.stopPropagation()}
      captionProps={{
        onPointerDown: (e) => beginWindowDrag(win.id, e),
        onDoubleClick: maximizable ? handle.toggleMaximize : undefined,
      }}
      overlay={resizable && !win.maximized && <ResizeHandles onResizeStart={onResizeStart} />}
    >
      <WindowContext.Provider value={handle}>
        {win.extendGlass ? (
          win.content
        ) : (
          <WindowBody surface={surface}>{win.content}</WindowBody>
        )}
      </WindowContext.Provider>
    </WindowFrame>
  );
}
