import { openWindow, type WindowHandle } from "./manager.js";
import { showMessageBox } from "./message-box.js";
import type { RuntimeBridge } from "../runtime-bridge.js";
import type { UiEvent } from "../worker.js";

const WM_CLOSE = 0x0010;

interface GuestWindow {
  win: WindowHandle;
  canvas: HTMLCanvasElement;
  ctx2d: CanvasRenderingContext2D;
  destroyed: boolean;
}
const windows = new Map<string, GuestWindow>();

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

export function handleUiEvents(pid: number, events: UiEvent[], runtime: RuntimeBridge) {
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
        const g = windows.get(key(pid, ev.hwnd));
        if (g) g.win.el.style.display = ev.show ? "" : "none";
        break;
      }
      case "set_window_text": {
        const g = windows.get(key(pid, ev.hwnd));
        if (g) g.win.setTitle(ev.title);
        break;
      }
      case "destroy_window": {
        const g = windows.get(key(pid, ev.hwnd));
        if (g) { g.destroyed = true; g.win.close(); windows.delete(key(pid, ev.hwnd)); }
        break;
      }
      default: {
        const g = windows.get(key(pid, ev.hwnd as number));
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
      c.ellipse(ev.x + ev.w / 2, ev.y + ev.h / 2, Math.abs(ev.w / 2), Math.abs(ev.h / 2), 0, 0, Math.PI * 2);
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
  if (windows.has(k)) return;

  let entry: GuestWindow;
  openWindow({
    title: ev.title || "Window",
    icon: "🪟",
    variant: "window",
    width: Math.max(ev.width, 200),
    height: Math.max(ev.height, 120) + 30,
    render: (win) => {
      const canvas = document.createElement("canvas");
      canvas.className = "guestwin-canvas";
      canvas.width = ev.width;
      canvas.height = ev.height;
      win.body.append(canvas);
      const ctx2d = canvas.getContext("2d")!;
      ctx2d.fillStyle = "#fff";
      ctx2d.fillRect(0, 0, canvas.width, canvas.height);
      entry = { win, canvas, ctx2d, destroyed: false };
      windows.set(k, entry);
      return () => {
        if (!entry.destroyed) runtime.postWindowMessage(pid, ev.hwnd, WM_CLOSE);
        windows.delete(k);
      };
    },
  });
}

// ── Web Audio beep ───────────────────────────────────────────────────────────
let audioCtx: AudioContext | null = null;
function beep(freq: number, durationMs: number) {
  try {
    audioCtx ??= new AudioContext();
    const osc = audioCtx.createOscillator();
    const gain = audioCtx.createGain();
    osc.type = "square";
    osc.frequency.value = freq > 0 ? freq : 800;
    gain.gain.value = 0.08;
    osc.connect(gain).connect(audioCtx.destination);
    const now = audioCtx.currentTime;
    osc.start(now);
    osc.stop(now + Math.min(Math.max(durationMs, 50), 2000) / 1000);
  } catch { /* audio not available */ }
}
