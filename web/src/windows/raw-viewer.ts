import { createWindow } from "./manager.js";
import type { RuntimeBridge } from "../runtime-bridge.js";

export function openRawViewer(path: string, runtime: RuntimeBridge) {
  const fileName = path.split("\\").pop() ?? path;
  const { body, setTitle } = createWindow({ title: fileName, width: 640, height: 460 });

  setTitle(`${fileName} — Raw Viewer`);

  const pre = document.createElement("pre");
  pre.className = "raw-viewer-content";
  pre.textContent = "Loading…";
  body.appendChild(pre);

  runtime.readFile(path).then((bytes) => {
    const decoder = new TextDecoder("utf-8", { fatal: true });
    try {
      pre.textContent = decoder.decode(bytes);
    } catch {
      pre.textContent = toHexDump(bytes);
    }
  }).catch((err) => {
    pre.textContent = `Error: ${err}`;
  });
}

function toHexDump(bytes: Uint8Array): string {
  const lines: string[] = [];
  for (let i = 0; i < bytes.length; i += 16) {
    const chunk = bytes.slice(i, i + 16);
    const hex = Array.from(chunk)
      .map((b) => b.toString(16).padStart(2, "0"))
      .join(" ");
    const ascii = Array.from(chunk)
      .map((b) => (b >= 0x20 && b < 0x7f ? String.fromCharCode(b) : "."))
      .join("");
    lines.push(`${i.toString(16).padStart(8, "0")}  ${hex.padEnd(47)}  ${ascii}`);
    if (lines.length > 2048) {
      lines.push("… (truncated)");
      break;
    }
  }
  return lines.join("\n");
}
