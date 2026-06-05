import { useEffect, useMemo, useRef } from "react";
import { useWindowStore } from "@/state/windowStore";

import type { UiEvent } from "@/core/wasm/worker";

// MB button groups
const MB_OK = 0x00000000;
const MB_OKCANCEL = 0x00000001;
const MB_ABORTRETRYIGNORE = 0x00000002;
const MB_YESNOCANCEL = 0x00000003;
const MB_YESNO = 0x00000004;
const MB_RETRYCANCEL = 0x00000005;
const MB_CANCELTRYCONTINUE = 0x00000006;

// MB default button flags
const MB_DEFBUTTON1 = 0x00000000;
const MB_DEFBUTTON2 = 0x00000100;
const MB_DEFBUTTON3 = 0x00000200;
const MB_DEFBUTTON4 = 0x00000300;

// MB icon flags
const MB_ICONHAND = 0x10; // error / stop
const MB_ICONQUESTION = 0x20;
const MB_ICONEXCLAMATION = 0x30; // warning
const MB_ICONASTERISK = 0x40; // information

// Win32-style result IDs
export type MessageBoxResult =
  | "ok"
  | "cancel"
  | "abort"
  | "retry"
  | "ignore"
  | "yes"
  | "no"
  | "tryagain"
  | "continue";

type MessageBoxButton = {
  id: MessageBoxResult;
  label: string;
  cancel?: boolean;
};

function iconFor(style: number): string {
  switch (style & 0xf0) {
    case MB_ICONHAND:
      return `${import.meta.env.BASE_URL}theme/icons/shell/msg_error.webp`;
    case MB_ICONQUESTION:
      return `${import.meta.env.BASE_URL}theme/icons/shell/msg_question.webp`;
    case MB_ICONEXCLAMATION:
      return `${import.meta.env.BASE_URL}theme/icons/shell/msg_warning.webp`;
    case MB_ICONASTERISK:
      return `${import.meta.env.BASE_URL}theme/icons/shell/msg_inform.webp`;
    default:
      return `${import.meta.env.BASE_URL}theme/icons/shell/msg_inform.webp`;
  }
}

function buttonsFor(style: number): MessageBoxButton[] {
  switch (style & 0x0000000f) {
    case MB_OKCANCEL:
      return [
        { id: "ok", label: "OK" },
        { id: "cancel", label: "Cancel", cancel: true },
      ];

    case MB_ABORTRETRYIGNORE:
      return [
        { id: "abort", label: "Abort" },
        { id: "retry", label: "Retry" },
        { id: "ignore", label: "Ignore" },
      ];

    case MB_YESNOCANCEL:
      return [
        { id: "yes", label: "Yes" },
        { id: "no", label: "No" },
        { id: "cancel", label: "Cancel", cancel: true },
      ];

    case MB_YESNO:
      return [
        { id: "yes", label: "Yes" },
        { id: "no", label: "No", cancel: true },
      ];

    case MB_RETRYCANCEL:
      return [
        { id: "retry", label: "Retry" },
        { id: "cancel", label: "Cancel", cancel: true },
      ];

    case MB_CANCELTRYCONTINUE:
      return [
        { id: "cancel", label: "Cancel", cancel: true },
        { id: "tryagain", label: "Try Again" },
        { id: "continue", label: "Continue" },
      ];

    case MB_OK:
    default:
      return [{ id: "ok", label: "OK", cancel: true }];
  }
}

function defaultButtonIndex(style: number, buttons: MessageBoxButton[]): number {
  const flag = style & 0x00000300;

  if (flag === MB_DEFBUTTON2) return Math.min(1, buttons.length - 1);
  if (flag === MB_DEFBUTTON3) return Math.min(2, buttons.length - 1);
  if (flag === MB_DEFBUTTON4) return Math.min(3, buttons.length - 1);

  return 0;
}

function cancelButton(buttons: MessageBoxButton[]): MessageBoxButton {
  return buttons.find((button) => button.cancel) ?? buttons[buttons.length - 1] ?? {
    id: "ok",
    label: "OK",
    cancel: true,
  };
}

export function showMessageBox(
  ev: Extract<UiEvent, { kind: "message_box" }>,
): Promise<MessageBoxResult> {
  return new Promise((resolve) => {
    let winId = "";

    const closeWith = (result: MessageBoxResult) => {
      if (winId) {
        useWindowStore.getState().closeWindow(winId);
      }

      resolve(result);
    };

    winId = useWindowStore.getState().openWindow({
      title: ev.title || "Message",
      icon: iconFor(ev.style),
      variant: "dialog",
      width: 420,
      content: <MessageBoxApp ev={ev} onClose={closeWith} />,
    });
  });
}

function MessageBoxApp({
  ev,
  onClose,
}: {
  ev: Extract<UiEvent, { kind: "message_box" }>;
  onClose: (result: MessageBoxResult) => void;
}) {
  const buttons = useMemo(() => buttonsFor(ev.style), [ev.style]);
  const defaultIndex = useMemo(() => defaultButtonIndex(ev.style, buttons), [ev.style, buttons]);
  const defaultButton = buttons[defaultIndex] ?? buttons[0];
  const cancel = useMemo(() => cancelButton(buttons), [buttons]);

  const buttonRefs = useRef<Array<HTMLButtonElement | null>>([]);

  useEffect(() => {
    buttonRefs.current[defaultIndex]?.focus();

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        onClose(defaultButton.id);
      }

      if (e.key === "Escape") {
        e.preventDefault();
        onClose(cancel.id);
      }
    };

    document.addEventListener("keydown", onKey);

    return () => {
      document.removeEventListener("keydown", onKey);
    };
  }, [cancel.id, defaultButton.id, defaultIndex, onClose]);

  return (
    <div className="flex flex-col h-full bg-[#202020] text-[#f2f2f2] font-[var(--system-font)]">
      <div className="flex flex-row gap-4 items-start flex-1 min-h-0 px-5 pt-5 pb-4 overflow-auto">
        <img
          src={iconFor(ev.style)}
          alt=""
          className="w-8 h-8 flex-none object-contain"
          draggable={false}
        />

        <div className="pt-[1px] text-[12px] leading-[17px] whitespace-pre-wrap break-words text-[#f2f2f2]">
          {ev.text}
        </div>
      </div>

      <div className="flex justify-end gap-2 px-4 py-3 bg-[#1b1b1b] border-t border-[#2d2d2d]">
        {buttons.map((button, index) => {
          const isDefault = index === defaultIndex;

          return (
            <button
              key={button.id}
              ref={(node) => {
                buttonRefs.current[index] = node;
              }}
              type="button"
              className={[
                "min-w-[76px] h-[26px] px-3",
                "rounded-none border text-[12px] font-normal font-[var(--system-font)]",
                "cursor-default outline-none",
                "bg-[#333333] text-[#f2f2f2] border-[#555555]",
                "hover:bg-[#3f3f3f] hover:border-[#6b6b6b]",
                "active:bg-[#2b2b2b]",
                "focus:border-[#0078d7] focus:shadow-[inset_0_0_0_1px_#0078d7]",
                isDefault
                  ? "border-[#0078d7] shadow-[inset_0_0_0_1px_#0078d7]"
                  : "",
              ].join(" ")}
              onClick={() => onClose(button.id)}
            >
              {button.label}
            </button>
          );
        })}
      </div>
    </div>
  );
}