import { createWindow } from "./manager.js";
import type { RuntimeBridge } from "../runtime-bridge.js";
import type { ProcessInfo } from "../worker.js";
import { log } from "../log.js";

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
  stdinInput.placeholder = "stdin (not yet enabled)";
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

  function appendLine(text: string, cls?: string) {
    for (const rawLine of text.split("\n")) {
      const line = rawLine.replace(/\r$/, "");
      if (line === "" && text.endsWith("\n")) continue;
      const span = document.createElement("span");
      if (cls) span.className = cls;
      span.textContent = line + "\n";
      pre.appendChild(span);
    }
    outputArea.scrollTop = outputArea.scrollHeight;
  }

  function appendRaw(text: string, cls?: string) {
    if (!text) return;
    const span = document.createElement("span");
    if (cls) span.className = cls;
    span.textContent = text;
    pre.appendChild(span);
    outputArea.scrollTop = outputArea.scrollHeight;
  }

  function setStatus(label: string) {
    const title = `${fileName}${label}`;
    setTitle(title);
    peBar.textContent = peBar.textContent!.replace(/ — .*$/, "") + ` — ${label}`;
  }

  runtime.launchProcess(path).then(({ pid, info }) => {
    const base = `0x${info.image_base.toString(16).toUpperCase().padStart(8, "0")}`;
    const entry = `0x${info.entry_point.toString(16).toUpperCase().padStart(8, "0")}`;
    peBar.textContent = `PID ${pid}  base ${base}  entry ${entry}`;
    setTitle(`${fileName} — PID ${pid} — Running`);
    killBtn.disabled = false;

    appendLine(`[loader] process created — PID ${pid}  base ${base}  entry ${entry}`, "pc-line-pe");

    // wire kill button
    killBtn.addEventListener("click", () => {
      runtime.killProcess(pid);
      killBtn.disabled = true;
      stdinInput.disabled = true;
      sendBtn.disabled = true;
      setStatus("Killed");
      appendLine("[process] killed by user", "pc-line-warn");
      log("process", `pid=${pid} killed by user`);
    });

    // wire stdin
    stdinInput.disabled = false;
    sendBtn.disabled = false;
    stdinInput.placeholder = "stdin";
    sendBtn.addEventListener("click", () => {
      const text = stdinInput.value;
      if (!text) return;
      stdinInput.value = "";
      appendLine(`> ${text}`, "pc-line-stdin");
      runtime.writeStdin(pid, text + "\n");
    });
    stdinInput.addEventListener("keydown", (e) => { if (e.key === "Enter") sendBtn.click(); });

    // register output callbacks
    runtime.onProcessOutput(pid, {
      stdout: (text) => appendRaw(text, "pc-line-out"),
      stderr: (text) => appendRaw(text, "pc-line-err-stream"),
      exited: (code) => {
        killBtn.disabled = true;
        stdinInput.disabled = true;
        sendBtn.disabled = true;
        setStatus(`Exited (${code})`);
        appendLine(`[process] exited with code ${code}`, code === 0 ? "pc-line-pe" : "pc-line-warn");
        log("process", `pid=${pid} exited code=${code}`);
      },
      crashed: (reason) => {
        killBtn.disabled = true;
        stdinInput.disabled = true;
        sendBtn.disabled = true;
        setStatus("Crashed");
        appendLine(`[cpu] crashed: ${reason}`, "pc-line-err");
        log("process", `pid=${pid} crashed: ${reason}`, "error");
      },
    });

    // start execution
    appendLine("[cpu] starting execution…", "pc-line-pe");
    runtime.runProcess(pid);

  }).catch((err) => {
    setTitle(`${fileName} — Error`);
    peBar.textContent = "Failed to load";
    appendLine(`[error] ${err}`, "pc-line-err");
    log("process", `failed to launch ${fileName}: ${err}`, "error");
  });
}
