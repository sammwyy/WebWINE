import "./MessageBoxApp.css";
import { useEffect, useRef } from "react";
import { useWindowStore } from "../../stores/useWindowStore.js";
import { useThemeStore } from "../../stores/useThemeStore.js";
import type { UiEvent } from "../../lib/worker.js";

// MB_* icon flags (0x70 = icon group).
const MB_ICONHAND = 0x10; // error / stop
const MB_ICONQUESTION = 0x20;
const MB_ICONEXCLAMATION = 0x30; // warning
const MB_ICONASTERISK = 0x40; // information

function iconFor(style: number, theme: string): string {
  switch (style & 0xf0) {
    case MB_ICONHAND:
      return `/themes/${theme}/icons/shell/msg_error.webp`;
    case MB_ICONQUESTION:
      return `/themes/${theme}/icons/shell/msg_question.webp`;
    case MB_ICONEXCLAMATION:
      return `/themes/${theme}/icons/shell/msg_warning.webp`;
    case MB_ICONASTERISK:
      return `/themes/${theme}/icons/shell/msg_inform.webp`;
    default:
      return `/themes/${theme}/icons/shell/msg_inform.webp`;
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

    const theme = useThemeStore.getState().theme;

    winId = useWindowStore.getState().openWindow({
      title: ev.title || "Message",
      icon: iconFor(ev.style, theme),
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
        <div className="msg-icon">
          <img src={iconFor(ev.style, useThemeStore.getState().theme)} alt="" style={{ width: 32, height: 32 }} draggable={false} />
        </div>
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
