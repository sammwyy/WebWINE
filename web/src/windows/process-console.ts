import { createWindow } from "./manager.js";
import type { RuntimeBridge } from "../runtime-bridge.js";
import type { ProcessInfo } from "../worker.js";

export function openProcessConsole(path: string, runtime: RuntimeBridge) {
  const fileName = path.split("\\").pop() ?? path;
  const { body, setTitle } = createWindow({
    title: `${fileName} — Launching…`,
    width: 680,
    height: 500,
  });

  body.className += " process-console-body";

  const peBar = document.createElement("div");
  peBar.className = "pc-pe-bar";
  peBar.textContent = "Loading PE…";

  const outputArea = document.createElement("div");
  outputArea.className = "pc-output";

  const pre = document.createElement("pre");
  pre.className = "pc-stream";
  outputArea.appendChild(pre);

  const footer = document.createElement("div");
  footer.className = "pc-footer";

  const stdinInput = document.createElement("input");
  stdinInput.className = "pc-stdin";
  stdinInput.type = "text";
  stdinInput.placeholder = "stdin";
  stdinInput.disabled = true;

  const sendBtn = document.createElement("button");
  sendBtn.className = "pc-btn";
  sendBtn.textContent = "Send";
  sendBtn.disabled = true;

  const killBtn = document.createElement("button");
  killBtn.className = "pc-btn pc-btn-danger";
  killBtn.textContent = "Kill";
  killBtn.disabled = true;

  footer.append(stdinInput, sendBtn, killBtn);
  body.append(peBar, outputArea, footer);

  function line(text: string, cls?: string) {
    const span = document.createElement("span");
    if (cls) span.className = cls;
    span.textContent = text + "\n";
    pre.appendChild(span);
    outputArea.scrollTop = outputArea.scrollHeight;
  }

  function stateLabel(info: ProcessInfo): string {
    const s = info.state;
    if (s.state === "exited")  return `Exited (${s.exit_code})`;
    if (s.state === "crashed") return `Crashed: ${s.reason}`;
    return s.state.charAt(0).toUpperCase() + s.state.slice(1);
  }

  function updateKillBtn(info: ProcessInfo) {
    const runnable = info.state.state === "created" || info.state.state === "running";
    killBtn.disabled = !runnable;
  }

  runtime.launchProcess(path).then(({ pid, info }) => {
    setTitle(`${fileName} — PID ${pid} — ${stateLabel(info)}`);

    peBar.textContent =
      `PID ${pid}  │  base 0x${info.image_base.toString(16).toUpperCase().padStart(8,"0")}` +
      `  entry 0x${info.entry_point.toString(16).toUpperCase().padStart(8,"0")}` +
      `  ${stateLabel(info)}`;

    updateKillBtn(info);

    killBtn.addEventListener("click", () => {
      runtime.killProcess(pid);
      killBtn.disabled = true;
      setTitle(`${fileName} — PID ${pid} — Killed`);
      peBar.textContent = peBar.textContent!.replace(/\w+$/, "Killed");
      line("[process] killed by user", "pc-line-warn");
    });

    // stdin send
    stdinInput.disabled = false;
    sendBtn.disabled = false;
    sendBtn.addEventListener("click", () => {
      const text = stdinInput.value;
      if (!text) return;
      stdinInput.value = "";
      line(`> ${text}`, "pc-line-stdin");
      // stdin write will be wired in Milestone 5
    });
    stdinInput.addEventListener("keydown", (e) => {
      if (e.key === "Enter") sendBtn.click();
    });

    // note about execution engine
    line(`[loader] process created — PID ${pid}`, "pc-line-pe");
    line(`[loader] EIP=0x${info.entry_point.toString(16).toUpperCase().padStart(8,"0")}`, "pc-line-pe");
    line(`[cpu] execution engine not yet available — Milestone 4`, "pc-line-warn");
  }).catch((err) => {
    setTitle(`${fileName} — Error`);
    peBar.textContent = "Failed to load";
    line(`[error] ${err}`, "pc-line-err");
  });
}
