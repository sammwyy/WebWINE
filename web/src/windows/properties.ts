import { createWindow } from "./manager.js";
import type { DirectoryEntry } from "../worker.js";

export function openProperties(entry: DirectoryEntry) {
  const { body, setTitle } = createWindow({
    title: `${entry.name} — Properties`,
    icon: "🛈",
    variant: "dialog",
    width: 360,
    height: 260,
  });

  setTitle(`${entry.name} — Properties`);

  const grid = document.createElement("div");
  grid.className = "props-grid";

  const ext = entry.name.includes(".")
    ? entry.name.split(".").pop()!.toUpperCase()
    : null;

  const typeLabel =
    entry.kind === "directory"
      ? "Folder"
      : ext
        ? `${ext} File`
        : "File";

  const rows: [string, string][] = [
    ["Name",     entry.name],
    ["Type",     typeLabel],
    ["Location", entry.path.split("\\").slice(0, -1).join("\\") || entry.path],
    ["Path",     entry.path],
  ];

  if (entry.kind === "file") {
    rows.push(["Size", formatSize(entry.size)]);
  }

  for (const [key, value] of rows) {
    const k = document.createElement("div");
    k.className = "props-key";
    k.textContent = key;

    const v = document.createElement("div");
    v.className = "props-val";
    v.textContent = value;

    grid.append(k, v);
  }

  body.appendChild(grid);
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} bytes`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
