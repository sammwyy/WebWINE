import { RuntimeBridge } from "./runtime-bridge.js";
import { refreshDesktop, initDesktopContextMenu } from "./desktop/icons.js";
import { initUpload } from "./desktop/upload.js";
import { log } from "./log.js";

const runtime = new RuntimeBridge();

document.getElementById("log-clear-btn")!.addEventListener("click", () => {
  const out = document.getElementById("log-output")!;
  out.innerHTML = "";
});

function updateClock() {
  const el = document.getElementById("taskbar-clock");
  if (el) el.textContent = new Date().toLocaleTimeString();
}
setInterval(updateClock, 1000);
updateClock();

async function main() {
  log("frontend", "Initializing WebWINE…");
  await runtime.ready();
  log("frontend", "Runtime ready");

  initUpload(runtime, () => refreshDesktop(runtime));
  initDesktopContextMenu(runtime);
  await refreshDesktop(runtime);
}

main().catch((err) => {
  log("frontend", `Fatal error: ${err}`, "error");
  console.error(err);
});
