/**
 * useWindowStore — manages all open windows on the desktop.
 *
 * Every surface (app windows, dialogs, guest windows) is a WindowRecord.
 * Components render their own content as React.ReactNode passed to openWindow.
 */

import { create } from "zustand";
import type React from "react";

export type WindowVariant = "window" | "dialog";

export interface WindowRect {
  left: string | number;
  top: string | number;
  width: string | number;
  height: string | number;
  transform: string;
}

export interface WindowRecord {
  id: string;
  title: string;
  icon?: string;
  variant: WindowVariant;
  resizable: boolean;
  hideTitlebar?: boolean;

  minimized: boolean;
  maximized: boolean;
  restoreRect?: WindowRect;

  zIndex: number;
  style: React.CSSProperties;

  content: React.ReactNode;
  onClose?: () => void;
}

export interface OpenWindowOptions {
  title: string;
  icon?: string;
  variant?: WindowVariant;
  width?: number;
  height?: number;
  resizable?: boolean;
  hideTitlebar?: boolean;
  content: React.ReactNode;
  /** Fired when the window is closed by any means (X button or `closeWindow`).
   * Used by modal dialogs to treat an X-close as a cancel. */
  onClose?: () => void;
}

export interface WindowTaskbarInfo {
  id: string;
  title: string;
  icon?: string;
  active: boolean;
  minimized: boolean;
  maximized: boolean;
}

let nextZIndex = 100;
let nextWindowId = 1;

function makeInitialStyle(
  variant: WindowVariant,
  w: number,
  h: number | undefined,
  id: number,
): React.CSSProperties {
  if (variant === "dialog") {
    return {
      width: w,
      ...(h !== undefined ? { height: h } : {}),
      left: "50%",
      top: "42%",
      transform: "translate(-50%,-50%)",
      zIndex: nextZIndex++,
    };
  }
  const x = 90 + (id % 6) * 28;
  const y = 60 + (id % 6) * 24;
  return {
    width: w,
    height: h ?? 420,
    left: x,
    top: y,
    zIndex: nextZIndex++,
  };
}

interface WindowStore {
  windows: WindowRecord[];
  activeId: string | null;

  openWindow: (opts: OpenWindowOptions) => string;
  closeWindow: (id: string) => void;
  focusWindow: (id: string) => void;
  minimizeWindow: (id: string) => void;
  maximizeWindow: (id: string) => void;
  restoreWindow: (id: string) => void;
  setTitle: (id: string, title: string) => void;
  setContent: (id: string, content: React.ReactNode) => void;
  updateStyle: (id: string, style: Partial<React.CSSProperties>) => void;
  activateFromTaskbar: (id: string) => void;

  getTaskbarInfo: () => WindowTaskbarInfo[];
}

export const useWindowStore = create<WindowStore>((set, get) => ({
  windows: [],
  activeId: null,

  openWindow: (opts) => {
    const id = `win-${nextWindowId++}`;
    const variant = opts.variant ?? "window";
    const isDialog = variant === "dialog";
    const resizable = opts.resizable ?? !isDialog;
    const w = opts.width ?? (isDialog ? 340 : 600);
    const h = opts.height;
    const style = makeInitialStyle(variant, w, h, nextWindowId);

    const record: WindowRecord = {
      id,
      title: opts.title,
      icon: opts.icon,
      variant,
      resizable,
      hideTitlebar: opts.hideTitlebar,
      minimized: false,
      maximized: false,
      zIndex: nextZIndex - 1,
      style,
      content: opts.content,
      onClose: opts.onClose,
    };

    set((state) => ({ windows: [...state.windows, record], activeId: id }));
    window.dispatchEvent(new Event("webwine:windows-changed"));
    return id;
  },

  closeWindow: (id) => {
    let closed: WindowRecord | undefined;
    set((state) => {
      closed = state.windows.find((w) => w.id === id);
      const remaining = state.windows.filter((w) => w.id !== id);
      const newActive =
        state.activeId === id ? topWindowId(remaining) : state.activeId;
      return { windows: remaining, activeId: newActive };
    });
    window.dispatchEvent(new Event("webwine:windows-changed"));
    closed?.onClose?.();
  },

  focusWindow: (id) => {
    set((state) => ({
      windows: state.windows.map((w) => {
        if (w.id === id) {
          const z = nextZIndex++;
          return {
            ...w,
            zIndex: z,
            style: { ...w.style, zIndex: z },
            minimized: false,
          };
        }
        return w;
      }),
      activeId: id,
    }));
    window.dispatchEvent(new Event("webwine:windows-changed"));
  },

  minimizeWindow: (id) => {
    set((state) => {
      const windows = state.windows.map((w) =>
        w.id === id ? { ...w, minimized: true } : w,
      );
      const newActive =
        state.activeId === id
          ? topWindowId(windows.filter((w) => !w.minimized))
          : state.activeId;
      return { windows, activeId: newActive };
    });
    window.dispatchEvent(new Event("webwine:windows-changed"));
  },

  maximizeWindow: (id) => {
    set((state) => ({
      windows: state.windows.map((w) => {
        if (w.id !== id || w.maximized) return w;
        const restoreRect: WindowRect = {
          left: w.style.left ?? "",
          top: w.style.top ?? "",
          width: w.style.width ?? "",
          height: w.style.height ?? "",
          transform: String(w.style.transform ?? ""),
        };
        const z = nextZIndex++;
        return {
          ...w,
          maximized: true,
          restoreRect,
          zIndex: z,
          style: {
            left: 0,
            top: 0,
            width: "100%",
            height: "100%",
            transform: "none",
            zIndex: z,
          },
        };
      }),
      activeId: id,
    }));
    window.dispatchEvent(new Event("webwine:windows-changed"));
  },

  restoreWindow: (id) => {
    set((state) => ({
      windows: state.windows.map((w) => {
        if (w.id !== id) return w;
        const r = w.restoreRect;
        const z = nextZIndex++;
        return {
          ...w,
          minimized: false,
          maximized: false,
          zIndex: z,
          style: r
            ? {
                left: r.left,
                top: r.top,
                width: r.width,
                height: r.height,
                transform: r.transform,
                zIndex: z,
              }
            : { ...w.style, zIndex: z },
        };
      }),
    }));
    window.dispatchEvent(new Event("webwine:windows-changed"));
  },

  setTitle: (id, title) => {
    set((state) => ({
      windows: state.windows.map((w) => (w.id === id ? { ...w, title } : w)),
    }));
    window.dispatchEvent(new Event("webwine:windows-changed"));
  },

  setContent: (id, content) => {
    set((state) => ({
      windows: state.windows.map((w) => (w.id === id ? { ...w, content } : w)),
    }));
    window.dispatchEvent(new Event("webwine:windows-changed"));
  },

  updateStyle: (id, style) => {
    set((state) => ({
      windows: state.windows.map((w) =>
        w.id === id ? { ...w, style: { ...w.style, ...style } } : w,
      ),
    }));
    window.dispatchEvent(new Event("webwine:windows-changed"));
  },

  activateFromTaskbar: (id) => {
    const { windows, activeId, minimizeWindow, restoreWindow, focusWindow } =
      get();
    const win = windows.find((w) => w.id === id);
    if (!win) return;

    if (win.minimized) {
      restoreWindow(id);
      focusWindow(id);
      return;
    }

    if (activeId === id) {
      minimizeWindow(id);
      return;
    }

    focusWindow(id);
  },

  getTaskbarInfo: () => {
    const { windows, activeId } = get();
    return windows.map((w) => ({
      id: w.id,
      title: w.title,
      icon: w.icon,
      active: w.id === activeId,
      minimized: w.minimized,
      maximized: w.maximized,
    }));
  },
}));

function topWindowId(windows: WindowRecord[]): string | null {
  let topId: string | null = null;
  let topZ = -1;
  for (const w of windows) {
    const z = w.zIndex || 0;
    if (z > topZ) {
      topZ = z;
      topId = w.id;
    }
  }
  return topId;
}
