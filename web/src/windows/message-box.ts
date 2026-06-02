import { openWindow } from "./manager.js";
import type { UiEvent } from "../worker.js";

// MB_* icon flags (0x70 = icon group).
const MB_ICONHAND        = 0x10; // error / stop
const MB_ICONQUESTION    = 0x20;
const MB_ICONEXCLAMATION = 0x30; // warning
const MB_ICONASTERISK    = 0x40; // information

function iconFor(style: number): string {
  switch (style & 0xF0) {
    case MB_ICONHAND:        return "⛔";
    case MB_ICONQUESTION:    return "❓";
    case MB_ICONEXCLAMATION: return "⚠️";
    case MB_ICONASTERISK:    return "ℹ️";
    default:                 return "ℹ️";
  }
}

// Render a MessageBox as a movable dialog window. Resolves when dismissed.
export function showMessageBox(ev: Extract<UiEvent, { kind: "message_box" }>): Promise<void> {
  return new Promise((resolve) => {
    const win = openWindow({
      title: ev.title || "Message",
      icon: iconFor(ev.style),
      variant: "dialog",
      width: 380,
      render: (win) => {
        const wrap = document.createElement("div");
        wrap.className = "dialog-content";

        const body = document.createElement("div");
        body.className = "msgbox-body";
        const icon = document.createElement("div");
        icon.className = "msgbox-icon";
        icon.textContent = iconFor(ev.style);
        const text = document.createElement("div");
        text.className = "msgbox-text";
        text.textContent = ev.text;
        body.append(icon, text);

        const buttons = document.createElement("div");
        buttons.className = "dialog-buttons";
        const ok = document.createElement("button");
        ok.className = "dialog-btn dialog-btn-default";
        ok.textContent = "OK";
        buttons.append(ok);

        wrap.append(body, buttons);
        win.body.append(wrap);

        const onKey = (e: KeyboardEvent) => {
          if (e.key === "Enter" || e.key === "Escape") win.close();
        };
        ok.addEventListener("click", () => win.close());
        document.addEventListener("keydown", onKey);
        requestAnimationFrame(() => ok.focus());

        return () => {
          document.removeEventListener("keydown", onKey);
          resolve();
        };
      },
    });
    void win;
  });
}
