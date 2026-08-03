import { useEffect, useRef, useState } from "react";
import { useWindowStore } from "@/state/windowStore";

import { showMessageBox, type MessageBoxResult } from "../message-box/MessageBoxApp";
import { showFileDialog } from "../file-dialog/FileDialogApp";
import { GuestMenuBar } from "./GuestMenuBar";
import type { MenuItemData } from "@/core/wasm/worker";

const WM_COMMAND = 0x0111;
import { beep } from "@/shared/lib/beep";
import type { RuntimeBridge } from "@/core/bridge/runtime-bridge";
import type { UiEvent } from "@/core/wasm/worker";
import { WebGLVideoDriver, type GpuCommand } from "@/core/gpu/video-driver";

const WM_CLOSE = 0x0010;

// MessageBox result string -> Win32 ID returned to the guest.
const MSG_RESULT_ID: Record<MessageBoxResult, number> = {
  ok: 1, cancel: 2, abort: 3, retry: 4, ignore: 5,
  yes: 6, no: 7, tryagain: 10, continue: 11,
};

// Since guest windows are updated imperatively via canvas calls, we maintain
// a global registry of the active canvas contexts and window IDs.
interface GuestWindowRecord {
  winId: string;
  canvas: HTMLCanvasElement;
  // A window is either GDI (2D canvas) or Direct3D8 (WebGL) — a canvas can hold
  // only one context type, so we pick lazily on the first event.
  ctx2d: CanvasRenderingContext2D | null;
  gl: WebGLVideoDriver | null;
  destroyed: boolean;
  queue: UiEvent[];
  menuItems: MenuItemData[];
  setMenuItems?: (items: MenuItemData[]) => void;
  dialogControls: import("@/core/wasm/worker").DialogControlData[];
  setDialogControls?: (items: import("@/core/wasm/worker").DialogControlData[]) => void;
}

/** Route one event to the window's 2D or WebGL backend, creating it lazily. */
function paint(rec: GuestWindowRecord, ev: UiEvent) {
  if (ev.kind.startsWith("gpu_")) {
    if (!rec.gl && !rec.ctx2d) {
      try {
        rec.gl = new WebGLVideoDriver(rec.canvas, rec.canvas.width, rec.canvas.height);
      } catch {
        return; // WebGL unavailable
      }
    }
    rec.gl?.submit(ev as GpuCommand);
    return;
  }
  if (!rec.ctx2d && !rec.gl) {
    rec.ctx2d = rec.canvas.getContext("2d");
    if (rec.ctx2d) {
      rec.ctx2d.fillStyle = "#fff";
      rec.ctx2d.fillRect(0, 0, rec.canvas.width, rec.canvas.height);
    }
  }
  if (rec.ctx2d) draw(rec.ctx2d, ev);
}

const guestWindows = new Map<string, GuestWindowRecord>();

function key(pid: number, hwnd: number): string {
  return `${pid}:${hwnd}`;
}

// COLORREF (0x00BBGGRR) -> CSS color.
function colorref(c: number): string {
  const r = c & 0xff;
  const g = (c >> 8) & 0xff;
  const b = (c >> 16) & 0xff;
  return `rgb(${r},${g},${b})`;
}

export function handleUiEvents(
  pid: number,
  events: UiEvent[],
  runtime: RuntimeBridge,
) {
  for (const ev of events) {
    switch (ev.kind) {
      case "message_box":
        // Modal: the guest is blocked; feed the clicked button back to resume it.
        void showMessageBox(ev).then((r) =>
          runtime.postDialogReply(pid, MSG_RESULT_ID[r] ?? 1),
        );
        break;
      case "file_dialog":
        // Modal: resume with the chosen path (button 1) or cancel (button 0).
        void showFileDialog(ev, runtime).then((path) =>
          runtime.postDialogReply(pid, path ? 1 : 0, path ?? ""),
        );
        break;
      case "set_menu": {
        const g = guestWindows.get(key(pid, ev.hwnd));
        if (g) {
          g.menuItems = ev.items;
          g.setMenuItems?.(ev.items);
        }
        break;
      }
      case "dialog_layout": {
        const g = guestWindows.get(key(pid, ev.hwnd));
        if (g) {
          g.dialogControls = ev.controls;
          g.setDialogControls?.([...ev.controls]);
        }
        break;
      }
      case "control_text": {
        const g = guestWindows.get(key(pid, ev.hwnd));
        if (g) {
          const ctrl = g.dialogControls.find((c) => c.hwnd === ev.control_hwnd);
          if (ctrl) {
            ctrl.text = ev.text;
            g.setDialogControls?.([...g.dialogControls]);
          }
        }
        break;
      }
      case "control_state": {
        const g = guestWindows.get(key(pid, ev.hwnd));
        if (g) {
          const ctrl = g.dialogControls.find((c) => c.hwnd === ev.control_hwnd);
          if (ctrl) {
            ctrl.enabled = ev.enabled;
            ctrl.checked = ev.checked;
            ctrl.visible = ev.visible;
            g.setDialogControls?.([...g.dialogControls]);
          }
        }
        break;
      }
      case "beep":
        beep(ev.freq, ev.duration);
        break;
      case "create_window":
        createGuestWindow(pid, ev, runtime);
        break;
      case "show_window": {
        const g = guestWindows.get(key(pid, ev.hwnd));
        // We don't directly toggle display inline anymore; we can toggle minimized
        // state or manage it via store if needed. For now, restoring/minimizing:
        if (g) {
          if (ev.show) useWindowStore.getState().restoreWindow(g.winId);
          else useWindowStore.getState().minimizeWindow(g.winId);
        }
        break;
      }
      case "set_window_text": {
        const g = guestWindows.get(key(pid, ev.hwnd));
        if (g) useWindowStore.getState().setTitle(g.winId, ev.title);
        break;
      }
      case "destroy_window": {
        const g = guestWindows.get(key(pid, ev.hwnd));
        if (g) {
          g.destroyed = true;
          useWindowStore.getState().closeWindow(g.winId);
          guestWindows.delete(key(pid, ev.hwnd));
        }
        break;
      }
      default: {
        const g = guestWindows.get(key(pid, ev.hwnd as number));
        if (g) {
          if (g.canvas) paint(g, ev);
          else g.queue.push(ev);
        }
      }
    }
  }
}

function draw(c: CanvasRenderingContext2D, ev: UiEvent) {
  switch (ev.kind) {
    case "clear_client":
      c.fillStyle = "#fff";
      c.fillRect(0, 0, c.canvas.width, c.canvas.height);
      break;
    case "draw_text":
      c.fillStyle = colorref(ev.color);
      c.font = "13px 'Segoe UI', system-ui, sans-serif";
      c.textBaseline = "top";
      c.fillText(ev.text, ev.x, ev.y);
      break;
    case "fill_rect":
      c.fillStyle = colorref(ev.color);
      c.fillRect(ev.x, ev.y, ev.w, ev.h);
      break;
    case "rect":
      c.fillStyle = colorref(ev.fill);
      c.strokeStyle = colorref(ev.stroke);
      c.fillRect(ev.x, ev.y, ev.w, ev.h);
      c.strokeRect(ev.x + 0.5, ev.y + 0.5, ev.w, ev.h);
      break;
    case "ellipse": {
      c.beginPath();
      c.ellipse(
        ev.x + ev.w / 2,
        ev.y + ev.h / 2,
        Math.abs(ev.w / 2),
        Math.abs(ev.h / 2),
        0,
        0,
        Math.PI * 2,
      );
      c.fillStyle = colorref(ev.fill);
      c.fill();
      c.strokeStyle = colorref(ev.stroke);
      c.stroke();
      break;
    }
    case "line":
      c.strokeStyle = colorref(ev.color);
      c.beginPath();
      c.moveTo(ev.x1 + 0.5, ev.y1 + 0.5);
      c.lineTo(ev.x2 + 0.5, ev.y2 + 0.5);
      c.stroke();
      break;
    case "set_pixel":
      c.fillStyle = colorref(ev.color);
      c.fillRect(ev.x, ev.y, 1, 1);
      break;
    case "blit": {
      // Framebuffer image (RGBA) from a DIB BitBlt/StretchDIBits.
      if (ev.src_w <= 0 || ev.src_h <= 0) break;
      if (ev.pixels.length < ev.src_w * ev.src_h * 4) break;
      const img = new ImageData(new Uint8ClampedArray(ev.pixels), ev.src_w, ev.src_h);
      if (ev.w === ev.src_w && ev.h === ev.src_h) {
        c.putImageData(img, ev.x, ev.y);
      } else {
        // Scaled blit: stage the source then drawImage into the target rect.
        blitScratch.width = ev.src_w;
        blitScratch.height = ev.src_h;
        const sc = blitScratch.getContext("2d");
        if (sc) {
          sc.putImageData(img, 0, 0);
          c.imageSmoothingEnabled = false;
          c.drawImage(blitScratch, 0, 0, ev.src_w, ev.src_h, ev.x, ev.y, ev.w, ev.h);
        }
      }
      break;
    }
  }
}

// Reused offscreen canvas for scaled blits (avoids per-frame allocation).
const blitScratch = document.createElement("canvas");

function createGuestWindow(
  pid: number,
  ev: Extract<UiEvent, { kind: "create_window" }>,
  runtime: RuntimeBridge,
) {
  const k = key(pid, ev.hwnd);
  if (guestWindows.has(k)) return;


  const winId = useWindowStore.getState().openWindow({
    title: ev.title || "Window",
    icon: `${import.meta.env.BASE_URL}theme/icons/shell/default_executable.webp`,
    variant: "window",
    width: Math.max(ev.width, 200),
    height: Math.max(ev.height, 120) + 30,
    content: <GuestWindowApp pid={pid} hwnd={ev.hwnd} ev={ev} runtime={runtime} />,
  });

  // We assign a placeholder record so events don't drop while React mounts it.
  // The actual canvas will be slotted in when the component mounts.
  guestWindows.set(k, {
    winId,
    canvas: null as unknown as HTMLCanvasElement,
    ctx2d: null,
    gl: null,
    destroyed: false,
    queue: [],
    menuItems: [],
    dialogControls: [],
  });
}

function GuestWindowApp({
  pid,
  hwnd,
  ev,
  runtime,
}: {
  pid: number;
  hwnd: number;
  ev: Extract<UiEvent, { kind: "create_window" }>;
  runtime: RuntimeBridge;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [menu, setMenu] = useState<MenuItemData[]>(
    () => guestWindows.get(key(pid, hwnd))?.menuItems ?? [],
  );
  const [controls, setControls] = useState<import("@/core/wasm/worker").DialogControlData[]>(
    () => guestWindows.get(key(pid, hwnd))?.dialogControls ?? []
  );

  useEffect(() => {
    const k = key(pid, hwnd);
    const rec = guestWindows.get(k);
    if (!rec || !canvasRef.current) return;

    rec.canvas = canvasRef.current;
    rec.setMenuItems = setMenu; // let live SetMenu events re-render the bar
    rec.setDialogControls = setControls;
    setMenu(rec.menuItems);
    setControls(rec.dialogControls);
    // Context (2D vs WebGL) is chosen lazily by the first queued/live event.
    for (const queuedEv of rec.queue) {
      paint(rec, queuedEv);
    }
    rec.queue = [];

    return () => {
      if (!rec.destroyed) {
        runtime.postWindowMessage(pid, hwnd, WM_CLOSE);
      }
      guestWindows.delete(k);
    };
  }, [pid, hwnd, runtime]);

  const bg = controls.length > 0 ? "#f0f0f0" : "var(--window-bg,#fff)";

  return (
    <div className="flex flex-col w-full h-full" style={{ backgroundColor: bg }}>
      <GuestMenuBar items={menu} onCommand={(id) => runtime.postWindowMessage(pid, hwnd, WM_COMMAND, id, 0)} />
      <div className="relative flex-1 overflow-hidden">
        <canvas
          ref={canvasRef}
          className="absolute inset-0 w-full h-full block"
          width={ev.width}
          height={ev.height}
        />
        {controls.map(c => {
          const cls = c.class_name.toLowerCase();
          const isBtn = cls === "button";
          const type = c.style & 0x0F;
          const isChk = isBtn && (type === 2 || type === 3);
          const isRad = isBtn && (type === 4 || type === 9);
          const isGrp = isBtn && type === 7;
          const isPush = isBtn && !isChk && !isRad && !isGrp;
          const isEdit = cls === "edit";
          
          return (
          <div
            key={c.hwnd}
            onClick={() => {
              if (isBtn) runtime.postWindowMessage(pid, hwnd, WM_COMMAND, c.id, c.hwnd);
            }}
            style={{
              position: "absolute",
              left: c.x,
              top: c.y,
              width: c.w,
              height: c.h,
              display: c.visible ? "flex" : "none",
              alignItems: isGrp ? "flex-start" : "center",
              justifyContent: isPush ? "center" : "flex-start",
              border: isPush ? "2px outset #dfdfdf" : isEdit ? "2px inset #dfdfdf" : isGrp ? "1px solid #a0a0a0" : "none",
              background: isPush ? "#e0e0e0" : isEdit ? "#fff" : "transparent",
              cursor: isBtn ? "pointer" : "default",
              userSelect: "none",
              padding: "2px",
              fontFamily: "'Segoe UI', system-ui, sans-serif",
              fontSize: "13px",
              color: "#000",
              boxSizing: "border-box",
              opacity: c.enabled ? 1 : 0.5,
              pointerEvents: c.enabled ? "auto" : "none",
            }}
          >
            {isChk && <input type="checkbox" checked={c.checked} readOnly className="mr-1" />}
            {isRad && <input type="radio" checked={c.checked} readOnly className="mr-1" />}
            {isGrp && <div style={{position:"absolute", top:-8, left:8, background:bg, padding:"0 4px"}}>{c.text.replace(/&/g, '')}</div>}
            {!isGrp && <span className="truncate" style={{width:"100%", textAlign: isPush ? "center" : "left"}}>{c.text.replace(/&/g, '')}</span>}
          </div>
        )})}
      </div>
    </div>
  );
}
