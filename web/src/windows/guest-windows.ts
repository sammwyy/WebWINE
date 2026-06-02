import { openWindow, type WindowHandle } from "./manager.js";
import { showMessageBox } from "./message-box.js";
import type { RuntimeBridge } from "../runtime-bridge.js";
import type { UiEvent } from "../worker.js";

const WM_CLOSE = 0x0010;

// Live guest windows keyed by "pid:hwnd".
interface GuestWindow {
  win: WindowHandle;
  client: HTMLElement;
  destroyed: boolean;
}
const windows = new Map<string, GuestWindow>();

function key(pid: number, hwnd: number): string {
  return `${pid}:${hwnd}`;
}

// Render a batch of guest UI events for a process.
export function handleUiEvents(pid: number, events: UiEvent[], runtime: RuntimeBridge) {
  for (const ev of events) {
    switch (ev.kind) {
      case "message_box":
        void showMessageBox(ev);
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
      case "clear_client": {
        const g = windows.get(key(pid, ev.hwnd));
        if (g) g.client.innerHTML = "";
        break;
      }
      case "draw_text": {
        const g = windows.get(key(pid, ev.hwnd));
        if (g) {
          const span = document.createElement("span");
          span.className = "guestwin-text";
          span.style.left = `${ev.x}px`;
          span.style.top = `${ev.y}px`;
          span.textContent = ev.text;
          g.client.append(span);
        }
        break;
      }
      case "destroy_window": {
        const g = windows.get(key(pid, ev.hwnd));
        if (g) { g.destroyed = true; g.win.close(); windows.delete(key(pid, ev.hwnd)); }
        break;
      }
    }
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

  const win = openWindow({
    title: ev.title || "Window",
    icon: "🪟",
    variant: "window",
    width: Math.max(ev.width, 200),
    height: Math.max(ev.height, 120) + 30, // + titlebar
    render: (win) => {
      const client = document.createElement("div");
      client.className = "guestwin-client";
      win.body.append(client);
      entry = { win, client, destroyed: false };
      windows.set(k, entry);
      // Closing the DOM window posts WM_CLOSE to the guest (which usually
      // destroys the window and quits its message loop).
      return () => {
        if (!entry.destroyed) {
          runtime.postWindowMessage(pid, ev.hwnd, WM_CLOSE);
        }
        windows.delete(k);
      };
    },
  });
  void win;
}
