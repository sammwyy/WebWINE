export function showInputDialog(opts: {
  title: string;
  placeholder?: string;
  initial?: string;
  onConfirm: (value: string) => void;
}) {
  const overlay = document.createElement("div");
  overlay.className = "dialog-overlay";

  const box = document.createElement("div");
  box.className = "dialog-box";

  const title = document.createElement("div");
  title.className = "dialog-title";
  title.textContent = opts.title;

  const input = document.createElement("input");
  input.className = "dialog-input";
  input.type = "text";
  input.placeholder = opts.placeholder ?? "";
  input.value = opts.initial ?? "";

  const buttons = document.createElement("div");
  buttons.className = "dialog-buttons";

  const okBtn = document.createElement("button");
  okBtn.className = "dialog-btn dialog-btn-ok";
  okBtn.textContent = "OK";

  const cancelBtn = document.createElement("button");
  cancelBtn.className = "dialog-btn";
  cancelBtn.textContent = "Cancel";

  buttons.append(okBtn, cancelBtn);
  box.append(title, input, buttons);
  overlay.appendChild(box);
  document.body.appendChild(overlay);

  requestAnimationFrame(() => {
    input.focus();
    input.select();
  });

  const confirm = () => {
    const value = input.value.trim();
    if (value) {
      overlay.remove();
      opts.onConfirm(value);
    }
  };

  const cancel = () => overlay.remove();

  okBtn.addEventListener("click", confirm);
  cancelBtn.addEventListener("click", cancel);
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") confirm();
    if (e.key === "Escape") cancel();
  });
  overlay.addEventListener("mousedown", (e) => {
    if (e.target === overlay) cancel();
  });
}
