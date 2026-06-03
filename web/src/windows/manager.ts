// Unified window framework. Every surface on the desktop — app windows,
// dialogs, message boxes, and (later) graphics/native process windows — is a
// movable window created here. Content is pluggable: a window is given a
// `WindowContent` whose `render` handler fills the body. The chrome (title bar,
// dragging, z-order, close) and the Win32-style look are shared.

let nextZIndex = 100;
let nextWindowId = 1;
let activeWindowId: string | null = null;

const openWindows = new Map<string, WindowRecord>();

export type WindowVariant = "window" | "dialog";

export interface WindowHandle {
  readonly el: HTMLElement;
  readonly body: HTMLElement;
  setTitle(title: string): void;
  minimize(): void;
  maximize(): void;
  restore(): void;
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

export interface WindowTaskbarInfo {
  id: string;
  title: string;
  icon?: string;
  active: boolean;
  minimized: boolean;
  maximized: boolean;
}

interface WindowRecord {
  id: string;
  title: string;
  icon?: string;
  el: HTMLElement;
  state: WindowState;
}

interface WindowState {
  minimized: boolean;
  maximized: boolean;
  restoreRect?: WindowRect;
}

interface WindowRect {
  left: string;
  top: string;
  width: string;
  height: string;
  transform: string;
}

export function getOpenWindows(): WindowTaskbarInfo[] {
  return Array.from(openWindows.values()).map((win) => ({
    id: win.id,
    title: win.title,
    icon: win.icon,
    active: win.id === activeWindowId,
    minimized: win.state.minimized,
    maximized: win.state.maximized,
  }));
}

export function focusWindowById(id: string) {
  const win = openWindows.get(id);
  if (!win) return;
  if (win.state.minimized) restoreWindow(win);
  focusWindow(win.el, id);
}

export function activateWindowFromTaskbar(id: string) {
  const win = openWindows.get(id);
  if (!win) return;

  if (win.state.minimized) {
    restoreWindow(win);
    focusWindow(win.el, id);
    return;
  }

  if (activeWindowId === id) {
    minimizeWindow(win);
    return;
  }

  focusWindow(win.el, id);
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
  openWindows.set(id, {
    id,
    title: content.title,
    icon: content.icon,
    el,
    state: {
      minimized: false,
      maximized: false,
    },
  });

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
  closeBtn.className = "window-control window-close";
  closeBtn.textContent = "✕";

  closeBtn.textContent = "";
  closeBtn.type = "button";
  closeBtn.title = "Close";
  closeBtn.setAttribute("aria-label", "Close");

  const controls = document.createElement("div");
  controls.className = "window-controls";

  const minBtn = createChromeButton("window-minimize", "Minimize");
  const maxBtn = createChromeButton("window-maximize", "Maximize");

  if (!isDialog) controls.append(minBtn, maxBtn);
  controls.append(closeBtn);

  titleBar.append(titleSpan, controls);

  const body = document.createElement("div");
  body.className = "window-body";

  el.append(titleBar, body);
  layer.appendChild(el);

  let cleanup: void | (() => void);
  const handle: WindowHandle = {
    el,
    body,
    setTitle: (t) => {
      titleSpan.textContent = t;
      const record = openWindows.get(id);
      if (record) record.title = t;
      dispatchWindowChange();
    },
    minimize: () => {
      const record = openWindows.get(id);
      if (record) minimizeWindow(record);
    },
    maximize: () => {
      const record = openWindows.get(id);
      if (record) maximizeWindow(record);
    },
    restore: () => {
      const record = openWindows.get(id);
      if (record) restoreWindow(record);
    },
    close: () => {
      if (typeof cleanup === "function") cleanup();
      openWindows.delete(id);
      if (activeWindowId === id) activeWindowId = getTopWindowId();
      el.remove();
      dispatchWindowChange();
    },
    focus: () => {
      focusWindow(el, id);
    },
  };

  closeBtn.onclick = () => handle.close();
  minBtn.onclick = () => handle.minimize();
  maxBtn.onclick = () => {
    const record = openWindows.get(id);
    if (!record) return;
    if (record.state.maximized) restoreWindow(record);
    else maximizeWindow(record);
  };
  titleBar.addEventListener("dblclick", (e) => {
    if (isDialog || (e.target as HTMLElement).closest(".window-controls")) return;
    const record = openWindows.get(id);
    if (!record) return;
    if (record.state.maximized) restoreWindow(record);
    else maximizeWindow(record);
  });
  makeDraggable(el, titleBar);
  el.addEventListener("mousedown", () => handle.focus());

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
    if ((e.target as HTMLElement).closest(".window-controls")) return;
    if (el.classList.contains("window--maximized")) return;
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

function focusWindow(el: HTMLElement, id: string) {
  const win = openWindows.get(id);
  if (win?.state.minimized) restoreWindow(win);
  el.style.zIndex = String(nextZIndex++);
  activeWindowId = id;
  dispatchWindowChange();
}

function getTopWindowId(): string | null {
  let topId: string | null = null;
  let topZ = -1;
  for (const win of openWindows.values()) {
    if (win.state.minimized) continue;
    const z = Number(win.el.style.zIndex) || 0;
    if (z > topZ) {
      topZ = z;
      topId = win.id;
    }
  }
  return topId;
}

function dispatchWindowChange() {
  window.dispatchEvent(new Event("webwine:windows-changed"));
}

function createChromeButton(className: string, label: string): HTMLButtonElement {
  const btn = document.createElement("button");
  btn.className = `window-control ${className}`;
  btn.type = "button";
  btn.title = label;
  btn.setAttribute("aria-label", label);
  return btn;
}

function minimizeWindow(win: WindowRecord) {
  win.state.minimized = true;
  win.el.classList.add("window--minimized");
  win.el.style.display = "none";
  if (activeWindowId === win.id) activeWindowId = getTopWindowId();
  dispatchWindowChange();
}

function maximizeWindow(win: WindowRecord) {
  if (win.state.maximized) return;

  win.state.restoreRect = {
    left: win.el.style.left,
    top: win.el.style.top,
    width: win.el.style.width,
    height: win.el.style.height,
    transform: win.el.style.transform,
  };

  win.state.maximized = true;
  win.el.classList.add("window--maximized");
  win.el.style.transform = "none";
  win.el.style.left = "0";
  win.el.style.top = "0";
  win.el.style.width = "100%";
  win.el.style.height = "100%";
  focusWindow(win.el, win.id);
}

function restoreWindow(win: WindowRecord) {
  if (win.state.minimized) {
    win.state.minimized = false;
    win.el.classList.remove("window--minimized");
    win.el.style.display = "";
  }

  if (win.state.maximized) {
    const rect = win.state.restoreRect;
    win.state.maximized = false;
    win.el.classList.remove("window--maximized");
    if (rect) {
      win.el.style.left = rect.left;
      win.el.style.top = rect.top;
      win.el.style.width = rect.width;
      win.el.style.height = rect.height;
      win.el.style.transform = rect.transform;
    }
  }

  dispatchWindowChange();
}
