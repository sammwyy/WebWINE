import { RuntimeBridge } from "./runtime-bridge.js";
import { refreshDesktop, initDesktopContextMenu, initDesktopThemeListener } from "./desktop/icons.js";
import { initUpload } from "./desktop/upload.js";
import { initShell } from "./desktop/shell.js";
import { initTheme } from "./desktop/theme.js";
import { log } from "./log.js";

initTheme();

const runtime = new RuntimeBridge();

document.getElementById("log-clear-btn")!.addEventListener("click", () => {
  const out = document.getElementById("log-output")!;
  out.innerHTML = "";
});

async function main() {
  log("frontend", "Initializing WebWINE…");
  await runtime.ready();
  log("frontend", "Runtime ready");

  initShell(runtime);
  initUpload(runtime, () => refreshDesktop(runtime));
  initDesktopContextMenu(runtime);
  initDesktopThemeListener(runtime);
  // Guest processes that create files emit this; reflect changes on the desktop.
  window.addEventListener("webwine:fs-changed", () => refreshDesktop(runtime));
  // A guest that calls CreateProcess gets a console window for its child.
  runtime.onProcessSpawned((pid, path) => {
    import("./windows/process-console.js").then((m) =>
      m.openProcessConsole(path, runtime, { attachPid: pid })
    );
  });
  await refreshDesktop(runtime);
}

main().catch((err) => {
  log("frontend", `Fatal error: ${err}`, "error");
  console.error(err);
});
