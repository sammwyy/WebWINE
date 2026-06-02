import { createWindow } from "./manager.js";
import type { RuntimeBridge } from "../runtime-bridge.js";
import { log } from "../log.js";

export function openProcessConsole(path: string, runtime: RuntimeBridge) {
  const fileName = path.split("\\").pop() ?? path;
  const { body, setTitle } = createWindow({
    title: `${fileName} — Launching…`,
    width: 660,
    height: 480,
  });

  body.className += " process-console-body";

  const peBar = document.createElement("div");
  peBar.className = "pc-pe-bar";
  peBar.textContent = "Reading PE headers…";

  const outputArea = document.createElement("div");
  outputArea.className = "pc-output";

  const stdoutPre = document.createElement("pre");
  stdoutPre.className = "pc-stream";

  outputArea.appendChild(stdoutPre);

  const footer = document.createElement("div");
  footer.className = "pc-footer";

  const stdinInput = document.createElement("input");
  stdinInput.className = "pc-stdin";
  stdinInput.type = "text";
  stdinInput.placeholder = "stdin (not yet available)";
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
    const span = document.createElement("span");
    if (cls) span.className = cls;
    span.textContent = text + "\n";
    stdoutPre.appendChild(span);
    outputArea.scrollTop = outputArea.scrollHeight;
  }

  runtime.inspectPe(path).then((info) => {
    setTitle(`${fileName} — ${info.machine} · ${info.subsystem} · Not runnable`);
    peBar.textContent =
      `${info.machine}  ${info.is_pe32 ? "PE32" : "PE32+"}  ${info.subsystem}` +
      `  │  base 0x${info.image_base.toString(16).toUpperCase().padStart(8, "0")}` +
      `  entry 0x${info.entry_point_rva.toString(16).toUpperCase().padStart(8, "0")}` +
      `  ${info.sections.length} sections  ${info.imports.reduce((n, m) => n + m.functions.length, 0)} imports`;

    appendLine(`[pe] ${fileName}: ${info.machine} ${info.subsystem}`, "pc-line-pe");
    appendLine(`[pe] image_base=0x${info.image_base.toString(16).toUpperCase().padStart(8,"0")}  entry_rva=0x${info.entry_point_rva.toString(16).toUpperCase().padStart(8,"0")}`, "pc-line-pe");
    appendLine(`[loader] execution engine not yet available — see Milestone 3`, "pc-line-warn");
    log("process", `run requested: ${fileName} (not yet executable)`);
  }).catch((err) => {
    setTitle(`${fileName} — Error`);
    peBar.textContent = "Failed to read PE headers";
    appendLine(`[error] ${err}`, "pc-line-err");
    log("process", `failed to inspect ${fileName}: ${err}`, "error");
  });
}
