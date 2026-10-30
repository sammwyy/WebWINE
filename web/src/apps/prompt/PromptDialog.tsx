/**
 * showPrompt — small modal text-input dialog (rename, new folder…) drawn as a
 * real themed dialog window. Resolves with the entered text, or null on cancel.
 */

import { useEffect, useRef, useState } from "react";
import { useWindowStore } from "@/state/windowStore";
import { Button, DialogFooter, Input } from "@/shared/ui";

export interface PromptOptions {
  title: string;
  label?: string;
  value?: string;
  icon?: string;
  okLabel?: string;
}

export function showPrompt(opts: PromptOptions): Promise<string | null> {
  return new Promise((resolve) => {
    let winId = "";
    let settled = false;
    const settle = (value: string | null) => {
      if (settled) return;
      settled = true;
      if (winId) useWindowStore.getState().closeWindow(winId);
      resolve(value);
    };

    winId = useWindowStore.getState().openWindow({
      title: opts.title,
      icon: opts.icon,
      variant: "dialog",
      width: 360,
      content: <PromptDialog opts={opts} onDone={settle} />,
      onClose: () => settle(null),
    });
  });
}

function PromptDialog({ opts, onDone }: { opts: PromptOptions; onDone: (value: string | null) => void }) {
  const [value, setValue] = useState(opts.value ?? "");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const input = inputRef.current;
    if (!input) return;
    input.focus();
    // Select the name without its extension, like Explorer does.
    const dot = input.value.lastIndexOf(".");
    input.setSelectionRange(0, dot > 0 ? dot : input.value.length);
  }, []);

  const submit = () => {
    const trimmed = value.trim();
    if (trimmed) onDone(trimmed);
  };

  return (
    <>
      <div className="flex flex-col gap-2 px-4 pt-4 pb-3">
        {opts.label && <label>{opts.label}</label>}
        <Input
          ref={inputRef}
          value={value}
          spellCheck={false}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") submit();
            if (e.key === "Escape") onDone(null);
          }}
        />
      </div>
      <DialogFooter>
        <Button isDefault onClick={submit} disabled={!value.trim()}>
          {opts.okLabel ?? "OK"}
        </Button>
        <Button onClick={() => onDone(null)}>Cancel</Button>
      </DialogFooter>
    </>
  );
}
