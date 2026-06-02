import { openWindow } from "../windows/manager.js";

// An input prompt rendered as a movable "dialog" window (no dark overlay).
export function showInputDialog(opts: {
  title: string;
  placeholder?: string;
  initial?: string;
  icon?: string;
  onConfirm: (value: string) => void;
}) {
  openWindow({
    title: opts.title,
    icon: opts.icon ?? "✏️",
    variant: "dialog",
    width: 360,
    render: (win) => {
      const wrap = document.createElement("div");
      wrap.className = "dialog-content";

      const input = document.createElement("input");
      input.className = "dialog-input";
      input.type = "text";
      input.placeholder = opts.placeholder ?? "";
      input.value = opts.initial ?? "";

      const buttons = document.createElement("div");
      buttons.className = "dialog-buttons";

      const okBtn = document.createElement("button");
      okBtn.className = "dialog-btn dialog-btn-default";
      okBtn.textContent = "OK";

      const cancelBtn = document.createElement("button");
      cancelBtn.className = "dialog-btn";
      cancelBtn.textContent = "Cancel";

      buttons.append(okBtn, cancelBtn);
      wrap.append(input, buttons);
      win.body.append(wrap);

      const confirm = () => {
        const value = input.value.trim();
        if (value) {
          win.close();
          opts.onConfirm(value);
        }
      };

      okBtn.addEventListener("click", confirm);
      cancelBtn.addEventListener("click", () => win.close());
      input.addEventListener("keydown", (e) => {
        if (e.key === "Enter") confirm();
        if (e.key === "Escape") win.close();
      });

      requestAnimationFrame(() => { input.focus(); input.select(); });
    },
  });
}
