import { createWindow } from "./manager.js";
import type { RuntimeBridge } from "../runtime-bridge.js";
import type { DirectoryEntry } from "../worker.js";

export function openExplorer(path: string, runtime: RuntimeBridge) {
  const { body, setTitle } = createWindow({ title: path, icon: "📁", width: 560, height: 400 });

  function navigate(target: string) {
    setTitle(target);
    body.innerHTML = "";

    const pathBar = document.createElement("div");
    pathBar.className = "explorer-pathbar";
    pathBar.textContent = target;
    body.appendChild(pathBar);

    const list = document.createElement("div");
    list.className = "explorer-list";
    body.appendChild(list);

    runtime.listDir(target).then((entries) => {
      if (entries.length === 0) {
        const empty = document.createElement("div");
        empty.className = "explorer-empty";
        empty.textContent = "(empty)";
        list.appendChild(empty);
        return;
      }

      for (const entry of entries) {
        list.appendChild(buildRow(entry, navigate, runtime));
      }
    }).catch((err) => {
      const msg = document.createElement("div");
      msg.className = "explorer-error";
      msg.textContent = `Error: ${err}`;
      list.appendChild(msg);
    });
  }

  navigate(path);
}

function buildRow(
  entry: DirectoryEntry,
  navigate: (p: string) => void,
  runtime: RuntimeBridge,
): HTMLElement {
  const row = document.createElement("div");
  row.className = "explorer-row";

  const icon = document.createElement("span");
  icon.className = "explorer-icon";
  icon.textContent = iconFor(entry);

  const name = document.createElement("span");
  name.className = "explorer-name";
  name.textContent = entry.name;

  const size = document.createElement("span");
  size.className = "explorer-size";
  size.textContent = entry.kind === "directory" ? "" : formatSize(entry.size);

  row.append(icon, name, size);

  row.addEventListener("dblclick", () => {
    if (entry.kind === "directory") {
      navigate(entry.path);
    } else if (entry.name.toLowerCase().endsWith(".exe")) {
      import("./pe-inspector.js").then((m) =>
        m.openPeInspector(entry.path, runtime)
      );
    } else {
      import("./raw-viewer.js").then((m) =>
        m.openRawViewer(entry.path, runtime)
      );
    }
  });

  return row;
}

function iconFor(entry: DirectoryEntry): string {
  if (entry.kind === "directory") return "📁";
  const name = entry.name.toLowerCase();
  if (name.endsWith(".exe")) return "⚙";
  if (name.endsWith(".txt") || name.endsWith(".log")) return "📄";
  return "📃";
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
