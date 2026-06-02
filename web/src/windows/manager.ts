// Unified window framework. Every surface on the desktop — app windows,
// dialogs, message boxes, and (later) graphics/native process windows — is a
// movable window created here. Content is pluggable: a window is given a
// `WindowContent` whose `render` handler fills the body. The chrome (title bar,
// dragging, z-order, close) and the Win32-style look are shared.

let nextZIndex = 100;
let nextWindowId = 1;

export type WindowVariant = "window" | "dialog";

export interface WindowHandle {
  readonly el: HTMLElement;
  readonly body: HTMLElement;
  setTitle(title: string): void;
  close(): void;
  focus(): void;
}

export interface WindowContent {
  title: string;
  /** Emoji/char shown in the title bar. */
  icon?: string;
  /** "window" (default, resizable app window) or "dialog" (compact, fixed). */
  variant?: WindowVariant;
  width?: number;
  height?: number;
  /** Defaults: true for windows, false for dialogs. */
  resizable?: boolean;
  /** Content handler — renders into `win.body`. May return a cleanup fn. */
  render(win: WindowHandle): void | (() => void);
}

export function openWindow(content: WindowContent): WindowHandle {
  const layer = document.getElementById("window-layer")!;
  const variant = content.variant ?? "window";
  const isDialog = variant === "dialog";
  const resizable = content.resizable ?? !isDialog;

  const w = content.width ?? (isDialog ? 340 : 600);
  const h = content.height; // dialogs may auto-height
  const id = `win-${nextWindowId++}`;

  const el = document.createElement("div");
  el.className = `window window--${variant}`;
  el.id = id;

  // Dialogs open centered; app windows cascade.
  if (isDialog) {
    el.style.cssText =
      `width:${w}px;${h ? `height:${h}px;` : ""}` +
      `left:50%;top:42%;transform:translate(-50%,-50%);z-index:${nextZIndex++}`;
  } else {
    const x = 90 + (nextWindowId % 6) * 28;
    const y = 60 + (nextWindowId % 6) * 24;
    el.style.cssText =
      `width:${w}px;height:${h ?? 420}px;left:${x}px;top:${y}px;z-index:${nextZIndex++}`;
  }
  if (resizable) el.classList.add("window--resizable");

  const titleBar = document.createElement("div");
  titleBar.className = "window-titlebar";

  if (content.icon) {
    const iconEl = document.createElement("span");
    iconEl.className = "window-icon";
    iconEl.textContent = content.icon;
    titleBar.append(iconEl);
  }

  const titleSpan = document.createElement("span");
  titleSpan.className = "window-title";
  titleSpan.textContent = content.title;

  const closeBtn = document.createElement("button");
  closeBtn.className = "window-close";
  closeBtn.textContent = "✕";

  titleBar.append(titleSpan, closeBtn);

  const body = document.createElement("div");
  body.className = "window-body";

  el.append(titleBar, body);
  layer.appendChild(el);

  let cleanup: void | (() => void);
  const handle: WindowHandle = {
    el,
    body,
    setTitle: (t) => { titleSpan.textContent = t; },
    close: () => {
      if (typeof cleanup === "function") cleanup();
      el.remove();
    },
    focus: () => {
      el.style.zIndex = String(nextZIndex++);
    },
  };

  closeBtn.onclick = () => handle.close();
  makeDraggable(el, titleBar);
  el.addEventListener("mousedown", () => { el.style.zIndex = String(nextZIndex++); });

  // Once positioned via transform (dialogs), drop the transform on first drag
  // so left/top math stays simple.
  cleanup = content.render(handle);
  handle.focus();
  return handle;
}

// Backwards-compatible imperative API used by existing windows. Creates a
// window with an empty body and hands it back for the caller to fill.
export function createWindow(opts: {
  title: string;
  icon?: string;
  variant?: WindowVariant;
  width?: number;
  height?: number;
  resizable?: boolean;
}): WindowHandle {
  let captured!: WindowHandle;
  openWindow({ ...opts, render: (win) => { captured = win; } });
  return captured;
}

function makeDraggable(el: HTMLElement, handle: HTMLElement) {
  handle.addEventListener("mousedown", (e) => {
    if ((e.target as HTMLElement).closest(".window-close")) return;
    e.preventDefault();

    // Resolve any centering transform into concrete left/top before dragging.
    const rect = el.getBoundingClientRect();
    el.style.transform = "none";
    el.style.left = `${rect.left}px`;
    el.style.top = `${rect.top}px`;

    const ox = e.clientX - rect.left;
    const oy = e.clientY - rect.top;

    function onMove(ev: MouseEvent) {
      el.style.left = `${ev.clientX - ox}px`;
      el.style.top = `${ev.clientY - oy}px`;
    }
    function onUp() {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
    }
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  });
}
