import { createWindow } from "./manager.js";
import type { RuntimeBridge } from "../runtime-bridge.js";
import { log } from "../log.js";

export function openProcessWindow(path: string, _runtime: RuntimeBridge) {
  const fileName = path.split("\\").pop() ?? path;
  const { body, setTitle } = createWindow({ title: fileName, width: 640, height: 480 });

  setTitle(`${fileName} — Not yet runnable`);

  const notice = document.createElement("div");
  notice.className = "process-notice";
  notice.textContent =
    "PE execution is not yet available (Milestone 4). " +
    "PE inspection will be added in Milestone 2.";

  const pathLine = document.createElement("div");
  pathLine.className = "process-path";
  pathLine.textContent = path;

  body.append(notice, pathLine);

  log("process", `attempted to launch: ${path} (not yet supported)`);
}
