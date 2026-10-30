import { useEffect, useMemo, useRef } from "react";
import { useWindowStore } from "@/state/windowStore";
import { Button, DialogFooter } from "@/shared/ui";

import type { UiEvent } from "@/core/wasm/worker";
import { iconUrl, playSound } from "@/shared/lib/personalization";
import type { SoundId } from "@/registry/sound-packs";

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
      return iconUrl("shell/msg_error");
    case MB_ICONQUESTION:
      return iconUrl("shell/msg_question");
    case MB_ICONEXCLAMATION:
      return iconUrl("shell/msg_warning");
    case MB_ICONASTERISK:
      return iconUrl("shell/msg_inform");
    default:
      return iconUrl("shell/msg_inform");
  }
}

function soundFor(style: number): SoundId {
  switch (style & 0xf0) {
    case MB_ICONHAND:
      return "error";
    case MB_ICONQUESTION:
      return "question";
    case MB_ICONEXCLAMATION:
      return "warning";
    case MB_ICONASTERISK:
      return "info";
    default:
      return "default";
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
    let settled = false;

    // Resolve once, whether from a button click or an X-close (→ cancel).
    const settle = (result: MessageBoxResult) => {
      if (settled) return;
      settled = true;
      if (winId) useWindowStore.getState().closeWindow(winId);
      resolve(result);
    };

    playSound(soundFor(ev.style));
    winId = useWindowStore.getState().openWindow({
      title: ev.title || "Message",
      icon: iconFor(ev.style),
      variant: "dialog",
      surface: "white",
      width: 420,
      content: <MessageBoxApp ev={ev} onClose={settle} />,
      onClose: () => settle(cancelButton(buttonsFor(ev.style)).id),
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
    <div className="flex flex-col h-full">
      <div className="flex gap-4 items-start flex-1 min-h-0 px-5 pt-5 pb-5 overflow-auto">
        <img
          src={iconFor(ev.style)}
          alt=""
          className="w-8 h-8 flex-none object-contain"
          draggable={false}
        />

        <div className="pt-[1px] leading-[17px] whitespace-pre-wrap break-words">{ev.text}</div>
      </div>

      <DialogFooter>
        {buttons.map((button, index) => (
          <Button
            key={button.id}
            ref={(node) => {
              buttonRefs.current[index] = node;
            }}
            isDefault={index === defaultIndex}
            onClick={() => onClose(button.id)}
          >
            {button.label}
          </Button>
        ))}
      </DialogFooter>
    </div>
  );
}