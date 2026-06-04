import { useEffect, useRef } from "react";
import { useWindowStore } from "@/state/windowStore";

import { showMessageBox } from "../message-box/MessageBoxApp";
import { beep } from "@/shared/lib/beep";
import type { RuntimeBridge } from "@/core/bridge/runtime-bridge";
import type { UiEvent } from "@/core/wasm/worker";

const WM_CLOSE = 0x0010;

// Since guest windows are updated imperatively via canvas calls, we maintain
// a global registry of the active canvas contexts and window IDs.
interface GuestWindowRecord {
  winId: string;
  canvas: HTMLCanvasElement;
  ctx2d: CanvasRenderingContext2D;
  destroyed: boolean;
  queue: UiEvent[];
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
        void showMessageBox(ev);
        break;
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
          if (g.ctx2d) draw(g.ctx2d, ev);
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
    icon: `/theme/icons/shell/default_executable.webp`,
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
    ctx2d: null as unknown as CanvasRenderingContext2D,
    destroyed: false,
    queue: [],
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

  useEffect(() => {
    const k = key(pid, hwnd);
    const rec = guestWindows.get(k);
    if (!rec || !canvasRef.current) return;

    rec.canvas = canvasRef.current;
    rec.ctx2d = canvasRef.current.getContext("2d")!;
    rec.ctx2d.fillStyle = "#fff";
    rec.ctx2d.fillRect(0, 0, rec.canvas.width, rec.canvas.height);

    for (const queuedEv of rec.queue) {
      draw(rec.ctx2d, queuedEv);
    }
    rec.queue = [];

    return () => {
      if (!rec.destroyed) {
        runtime.postWindowMessage(pid, hwnd, WM_CLOSE);
      }
      guestWindows.delete(k);
    };
  }, [pid, hwnd, runtime]);

  return (
    <canvas
      ref={canvasRef}
      className="w-full h-full block bg-[var(--window-bg,#fff)]"
      width={ev.width}
      height={ev.height}
    />
  );
}
