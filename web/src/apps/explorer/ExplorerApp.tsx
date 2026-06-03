import { useEffect, useState } from "react";
import { useWindowStore } from "../../stores/useWindowStore.js";
import type { RuntimeBridge } from "../../lib/runtime-bridge.js";
import type { DirectoryEntry } from "../../lib/worker.js";
import { formatSize } from "../../lib/utils.js";

export function openExplorer(initialPath: string, runtime: RuntimeBridge) {
  useWindowStore.getState().openWindow({
    title: initialPath,
    icon: "📁",
    width: 560,
    height: 400,
    content: <ExplorerApp initialPath={initialPath} runtime={runtime} />,
  });
}

function ExplorerApp({
  initialPath,
  runtime,
}: {
  initialPath: string;
  runtime: RuntimeBridge;
}) {
  const [path, setPath] = useState(initialPath);
  const [entries, setEntries] = useState<DirectoryEntry[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Find our window and update its title when path changes
    const store = useWindowStore.getState();
    const myWin = store.windows.find(
      (w) => (w.content as React.ReactElement<any>).props?.initialPath === initialPath, // somewhat hacky but works for now
    );
    // actually, better to just let the component not worry about title, or pass the window ID to the content.
    // I will refactor openWindow to pass the window ID as a prop to content if it's a function or we can just ignore title updating for now, or handle it via a hook.
    // For now, I'll fetch entries.
    runtime
      .listDir(path)
      .then((res) => {
        setEntries(res);
        setError(null);
      })
      .catch((err) => {
        setEntries([]);
        setError(String(err));
      });
  }, [path, runtime, initialPath]);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="explorer-pathbar">{path}</div>
      <div className="explorer-list" style={{ flex: 1, overflow: "auto" }}>
        {error && <div className="explorer-error">Error: {error}</div>}
        {!error && entries.length === 0 && (
          <div className="explorer-empty">(empty)</div>
        )}
        {!error &&
          entries.map((entry) => (
            <ExplorerRow
              key={entry.name}
              entry={entry}
              onNavigate={setPath}
              runtime={runtime}
            />
          ))}
      </div>
    </div>
  );
}

function ExplorerRow({
  entry,
  onNavigate,
  runtime,
}: {
  entry: DirectoryEntry;
  onNavigate: (path: string) => void;
  runtime: RuntimeBridge;
}) {
  const isDir = entry.kind === "directory";
  const icon = isDir
    ? "📁"
    : entry.name.toLowerCase().endsWith(".exe")
      ? "⚙"
      : entry.name.toLowerCase().endsWith(".txt") ||
          entry.name.toLowerCase().endsWith(".log")
        ? "📄"
        : "📃";

  return (
    <div
      className="explorer-row"
      onDoubleClick={() => {
        if (isDir) {
          onNavigate(entry.path);
        } else if (entry.name.toLowerCase().endsWith(".exe")) {
          import("../pe-inspector/PeInspectorApp.js").then((m) =>
            m.openPeInspector(entry.path, runtime),
          );
        } else {
          import("../text-reader/TextReaderApp.js").then((m) =>
            m.openTextReader(entry.path, runtime),
          );
        }
      }}
    >
      <span className="explorer-icon">{icon}</span>
      <span className="explorer-name">{entry.name}</span>
      <span className="explorer-size">
        {isDir ? "" : formatSize(entry.size)}
      </span>
    </div>
  );
}
