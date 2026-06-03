import { useEffect, useRef } from "react";
import { useWindowStore } from "../../stores/useWindowStore.js";
import { showMessageBox } from "../message-box/MessageBoxApp.js";
import { beep } from "../../lib/beep.js";
import type { RuntimeBridge } from "../../lib/runtime-bridge.js";
import type { UiEvent } from "../../lib/worker.js";

const WM_CLOSE = 0x0010;

// Since guest windows are updated imperatively via canvas calls, we maintain
// a global registry of the active canvas contexts and window IDs.
interface GuestWindowRecord {
  winId: string;
  canvas: HTMLCanvasElement;
  ctx2d: CanvasRenderingContext2D;
  destroyed: boolean;
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
        if (g) draw(g.ctx2d, ev);
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
  }
}

function createGuestWindow(
  pid: number,
  ev: Extract<UiEvent, { kind: "create_window" }>,
  runtime: RuntimeBridge,
) {
  const k = key(pid, ev.hwnd);
  if (guestWindows.has(k)) return;

  const winId = useWindowStore.getState().openWindow({
    title: ev.title || "Window",
    icon: "🪟",
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
      className="guestwin-canvas"
      width={ev.width}
      height={ev.height}
    />
  );
}
