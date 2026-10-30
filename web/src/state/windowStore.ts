/**
 * useWindowStore — manages all open windows on the desktop.
 *
 * Every surface (app windows, dialogs, guest windows) is a WindowRecord.
 * Components render their own content as React.ReactNode passed to openWindow.
 * The chrome (frame, caption, buttons, resize handles) is drawn by the active
 * theme — see modules/windows/AppWindow.tsx.
 */

import { create } from "zustand";
import type React from "react";
import { playSound } from "@/shared/lib/personalization";

export type WindowVariant = "window" | "dialog";

export interface WindowRect {
  x: number;
  y: number;
  width: number;
  /** `undefined` = size to content (dialogs without an explicit height) */
  height?: number;
}

/** Behaviour / appearance flags shared by `OpenWindowOptions` and `WindowRecord`. */
export interface WindowFeatures {
  /** allow resizing from the edges (default: true for windows, false for dialogs) */
  resizable?: boolean;
  /** show the minimize button (default: true for windows, false for dialogs) */
  minimizable?: boolean;
  /** show the maximize button (default: same as `resizable`) */
  maximizable?: boolean;
  /**
   * Draw no frame/caption at all: the content renders its own title bar
   * (e.g. with `<WindowTitlebar>`), like a custom-chrome app.
   */
  hideTitlebar?: boolean;
  /**
   * Extend the frame into the client area (like DwmExtendFrameIntoClientArea).
   * The content is rendered directly on the frame/glass and is responsible
   * for placing its own `<WindowBody>` / `<GlassPanel>` regions.
   */
  extendGlass?: boolean;
  /** background of the default client surface (ignored with `extendGlass`) */
  surface?: "white" | "face" | "none";
  minWidth?: number;
  minHeight?: number;
  /**
   * Requested size of the client area (e.g. a Win32 guest window). The outer
   * size is fitted once the themed frame is measured, since every theme has
   * different border / caption metrics.
   */
  clientSize?: { width: number; height: number };
}

export interface WindowRecord extends WindowRect, WindowFeatures {
  id: string;
  title: string;
  icon?: string;
  variant: WindowVariant;
  resizable: boolean;

  minimized: boolean;
  maximized: boolean;
  restoreRect?: WindowRect;

  zIndex: number;

  content: React.ReactNode;
  onClose?: () => void;
}

export interface OpenWindowOptions extends WindowFeatures {
  title: string;
  icon?: string;
  variant?: WindowVariant;
  width?: number;
  height?: number;
  x?: number;
  y?: number;
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

const CASCADE_STEP = 28;
const MIN_W = 180;
const MIN_H = 100;

let nextZIndex = 100;
let nextWindowId = 1;
let cascadeCounter = 0;

function initialRect(variant: WindowVariant, w: number, h: number | undefined): WindowRect {
  const vw = window.innerWidth;
  const vh = window.innerHeight;
  if (variant === "dialog") {
    return {
      x: Math.max(0, Math.round((vw - w) / 2)),
      y: Math.max(0, Math.round(vh * 0.42 - (h ?? 180) / 2)),
      width: w,
      height: h,
    };
  }
  const offset = (cascadeCounter++ % 8) * CASCADE_STEP;
  const height = h ?? 420;
  return {
    x: Math.max(0, Math.min(90 + offset, vw - w)),
    y: Math.max(0, Math.min(60 + offset, vh - height - 60)),
    width: w,
    height,
  };
}

function emitChanged() {
  window.dispatchEvent(new Event("webwine:windows-changed"));
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
  toggleMaximize: (id: string) => void;
  minimizeAll: () => void;
  moveWindow: (id: string, x: number, y: number) => void;
  setBounds: (id: string, rect: WindowRect) => void;
  getMinSize: (id: string) => { width: number; height: number };
  setTitle: (id: string, title: string) => void;
  setContent: (id: string, content: React.ReactNode) => void;
  /** change behaviour/appearance flags after creation (e.g. a guest window becoming a dialog) */
  updateWindow: (id: string, patch: WindowFeatures) => void;
  activateFromTaskbar: (id: string) => void;

  getTaskbarInfo: () => WindowTaskbarInfo[];
}

export const useWindowStore = create<WindowStore>((set, get) => {
  const update = (id: string, fn: (w: WindowRecord) => WindowRecord) =>
    set((state) => ({ windows: state.windows.map((w) => (w.id === id ? fn(w) : w)) }));

  return {
    windows: [],
    activeId: null,

    openWindow: (opts) => {
      const id = `win-${nextWindowId++}`;
      const { title, icon, variant = "window", width, height, x, y, content, onClose, ...features } = opts;
      const isDialog = variant === "dialog";
      const resizable = features.resizable ?? !isDialog;
      const rect = initialRect(variant, width ?? (isDialog ? 340 : 600), height);

      const record: WindowRecord = {
        ...features,
        id,
        title,
        icon,
        variant,
        resizable,
        minimizable: features.minimizable ?? !isDialog,
        maximizable: features.maximizable ?? resizable,
        ...rect,
        ...(x !== undefined ? { x } : {}),
        ...(y !== undefined ? { y } : {}),
        minimized: false,
        maximized: false,
        zIndex: nextZIndex++,
        content,
        onClose,
      };

      set((state) => ({ windows: [...state.windows, record], activeId: id }));
      emitChanged();
      if (!isDialog) playSound("open");
      return id;
    },

    closeWindow: (id) => {
      let closed: WindowRecord | undefined;
      set((state) => {
        closed = state.windows.find((w) => w.id === id);
        const remaining = state.windows.filter((w) => w.id !== id);
        const newActive = state.activeId === id ? topWindowId(remaining) : state.activeId;
        return { windows: remaining, activeId: newActive };
      });
      emitChanged();
      if (closed?.variant === "window") playSound("close");
      closed?.onClose?.();
    },

    focusWindow: (id) => {
      const { windows, activeId } = get();
      const win = windows.find((w) => w.id === id);
      if (!win) return;
      if (activeId === id && !win.minimized && win.zIndex === nextZIndex - 1) return;
      const z = nextZIndex++;
      set((state) => ({
        windows: state.windows.map((w) => (w.id === id ? { ...w, zIndex: z, minimized: false } : w)),
        activeId: id,
      }));
      emitChanged();
    },

    minimizeWindow: (id) => {
      set((state) => {
        const windows = state.windows.map((w) => (w.id === id ? { ...w, minimized: true } : w));
        const newActive =
          state.activeId === id ? topWindowId(windows.filter((w) => !w.minimized)) : state.activeId;
        return { windows, activeId: newActive };
      });
      emitChanged();
      playSound("minimize");
    },

    maximizeWindow: (id) => {
      const z = nextZIndex++;
      update(id, (w) =>
        w.maximized
          ? w
          : {
              ...w,
              maximized: true,
              minimized: false,
              restoreRect: { x: w.x, y: w.y, width: w.width, height: w.height },
              zIndex: z,
            },
      );
      set({ activeId: id });
      emitChanged();
      playSound("maximize");
    },

    restoreWindow: (id) => {
      const z = nextZIndex++;
      const win = get().windows.find((w) => w.id === id);
      if (win && (win.maximized || win.minimized)) playSound("restore");
      update(id, (w) => ({
        ...w,
        ...(w.maximized && w.restoreRect ? w.restoreRect : {}),
        minimized: false,
        maximized: false,
        zIndex: z,
      }));
      emitChanged();
    },

    toggleMaximize: (id) => {
      const win = get().windows.find((w) => w.id === id);
      if (!win) return;
      if (win.maximized) get().restoreWindow(id);
      else get().maximizeWindow(id);
    },

    minimizeAll: () => {
      set((state) => ({
        windows: state.windows.map((w) => ({ ...w, minimized: true })),
        activeId: null,
      }));
      emitChanged();
    },

    moveWindow: (id, x, y) => {
      update(id, (w) => ({ ...w, x, y }));
    },

    getMinSize: (id) => {
      const win = get().windows.find((w) => w.id === id);
      return { width: win?.minWidth ?? MIN_W, height: win?.minHeight ?? MIN_H };
    },

    setBounds: (id, rect) => {
      const min = get().getMinSize(id);
      update(id, (w) => ({
        ...w,
        ...rect,
        width: Math.max(min.width, rect.width),
        height: rect.height === undefined ? undefined : Math.max(min.height, rect.height),
      }));
    },

    setTitle: (id, title) => {
      update(id, (w) => ({ ...w, title }));
      emitChanged();
    },

    setContent: (id, content) => {
      update(id, (w) => ({ ...w, content }));
      emitChanged();
    },

    updateWindow: (id, patch) => {
      update(id, (w) => ({ ...w, ...patch, resizable: patch.resizable ?? w.resizable }));
    },

    activateFromTaskbar: (id) => {
      const { windows, activeId, minimizeWindow, focusWindow } = get();
      const win = windows.find((w) => w.id === id);
      if (!win) return;
      if (!win.minimized && activeId === id) minimizeWindow(id);
      else focusWindow(id);
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
  };
});

function topWindowId(windows: WindowRecord[]): string | null {
  let topId: string | null = null;
  let topZ = -1;
  for (const w of windows) {
    if (w.zIndex > topZ) {
      topZ = w.zIndex;
      topId = w.id;
    }
  }
  return topId;
}
