import { useEffect, useRef } from "react";
import { useWindowStore } from "../../stores/useWindowStore.js";
import type { UiEvent } from "../../lib/worker.js";

// MB_* icon flags (0x70 = icon group).
const MB_ICONHAND = 0x10; // error / stop
const MB_ICONQUESTION = 0x20;
const MB_ICONEXCLAMATION = 0x30; // warning
const MB_ICONASTERISK = 0x40; // information

function iconFor(style: number): string {
  switch (style & 0xf0) {
    case MB_ICONHAND:
      return "⛔";
    case MB_ICONQUESTION:
      return "❓";
    case MB_ICONEXCLAMATION:
      return "⚠️";
    case MB_ICONASTERISK:
      return "ℹ️";
    default:
      return "ℹ️";
  }
}

export function showMessageBox(
  ev: Extract<UiEvent, { kind: "message_box" }>,
): Promise<void> {
  return new Promise((resolve) => {
    let winId: string;
    const onClose = () => {
      useWindowStore.getState().closeWindow(winId);
      resolve();
    };

    winId = useWindowStore.getState().openWindow({
      title: ev.title || "Message",
      icon: iconFor(ev.style),
      variant: "dialog",
      width: 380,
      content: <MessageBoxApp ev={ev} onClose={onClose} />,
    });
  });
}

function MessageBoxApp({
  ev,
  onClose,
}: {
  ev: Extract<UiEvent, { kind: "message_box" }>;
  onClose: () => void;
}) {
  const btnRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    btnRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Enter" || e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className="dialog-content">
      <div className="msgbox-body">
        <div className="msgbox-icon">{iconFor(ev.style)}</div>
        <div className="msgbox-text">{ev.text}</div>
      </div>
      <div className="dialog-buttons">
        <button
          ref={btnRef}
          type="button"
          className="dialog-btn dialog-btn-default"
          onClick={onClose}
        >
          OK
        </button>
      </div>
    </div>
  );
}
