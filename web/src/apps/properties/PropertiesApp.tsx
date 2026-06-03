import { useWindowStore } from "../../stores/useWindowStore.js";
import type { DirectoryEntry } from "../../lib/worker.js";
import { formatSize, basename } from "../../lib/utils.js";

export function openProperties(entry: DirectoryEntry) {
  useWindowStore.getState().openWindow({
    title: `${entry.name} — Properties`,
    icon: "🛈",
    variant: "dialog",
    width: 360,
    height: 260,
    content: <PropertiesApp entry={entry} />,
  });
}

function PropertiesApp({ entry }: { entry: DirectoryEntry }) {
  const ext = entry.name.includes(".")
    ? entry.name.split(".").pop()!.toUpperCase()
    : null;

  const typeLabel =
    entry.kind === "directory" ? "Folder" : ext ? `${ext} File` : "File";

  const location = entry.path.split("\\").slice(0, -1).join("\\") || entry.path;

  return (
    <div className="props-grid">
      <div className="props-key">Name</div>
      <div className="props-val">{entry.name}</div>

      <div className="props-key">Type</div>
      <div className="props-val">{typeLabel}</div>

      <div className="props-key">Location</div>
      <div className="props-val">{location}</div>

      <div className="props-key">Path</div>
      <div className="props-val">{entry.path}</div>

      {entry.kind === "file" && (
        <>
          <div className="props-key">Size</div>
          <div className="props-val">{formatSize(entry.size)}</div>
        </>
      )}
    </div>
  );
}
