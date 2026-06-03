import "./ExplorerApp.css";
import { useEffect, useMemo, useState } from "react";
import { useWindowStore } from "../../stores/useWindowStore.js";
import type { RuntimeBridge } from "../../lib/runtime-bridge.js";
import type { DirectoryEntry } from "../../lib/worker.js";
import { formatSize } from "../../lib/utils.js";
import { useThemeStore } from "../../stores/useThemeStore.js";
import { launchGuestPath } from "../../lib/guest-launch.js";

const ROOT_PATH = "";
const DRIVE_PATH = "C:\\";
const GUEST_HOME = "C:\\Users\\guest";
const DESKTOP_PATH = `${GUEST_HOME}\\Desktop`;

const USER_FOLDERS = [
  { label: "Desktop", path: DESKTOP_PATH, place: "desktop" },
  { label: "Documents", path: `${GUEST_HOME}\\Documents`, place: "documents" },
  { label: "Pictures", path: `${GUEST_HOME}\\Pictures`, place: "pictures" },
  { label: "Music", path: `${GUEST_HOME}\\Music`, place: "music" },
  { label: "Videos", path: `${GUEST_HOME}\\Videos`, place: "video" },
] as const;

const ROOT_ENTRIES: DirectoryEntry[] = [
  {
    name: "C:",
    path: DRIVE_PATH,
    kind: "directory",
    size: 0,
  },
];

type SidebarSection = {
  section: string;
  items: { label: string; path: string; icon: string }[];
};

export function openExplorer(initialPath = ROOT_PATH, runtime: RuntimeBridge) {
  const theme = useThemeStore.getState().theme;
  const title = initialPath ? `File Explorer - ${initialPath}` : "File Explorer - Your PC";
  const id = useWindowStore.getState().openWindow({
    title,
    icon: `/themes/${theme}/icons/apps/explorer.webp`,
    width: 780,
    height: 500,
    content: <div />,
  });

  useWindowStore.getState().setContent(
    id,
    <ExplorerApp initialPath={initialPath} runtime={runtime} windowId={id} />,
  );
}

function ExplorerApp({
  initialPath,
  runtime,
  windowId,
}: {
  initialPath: string;
  runtime: RuntimeBridge;
  windowId: string;
}) {
  const { theme } = useThemeStore();
  const { setTitle } = useWindowStore();
  const [nav, setNav] = useState<{ history: string[]; index: number }>(() => {
    const root = normalizePath(initialPath);
    return { history: [root], index: 0 };
  });
  const path = nav.history[nav.index] ?? ROOT_PATH;
  const [address, setAddress] = useState(displayPath(path));
  const [entries, setEntries] = useState<DirectoryEntry[]>([]);
  const [error, setError] = useState<string | null>(null);

  const sections = useMemo<SidebarSection[]>(
    () => [
      {
        section: "Quick access",
        items: [
          { label: "Your PC", path: ROOT_PATH, icon: `/themes/${theme}/icons/places/thispc.webp` },
          ...USER_FOLDERS.map((f) => ({
            label: f.label,
            path: f.path,
            icon: `/themes/${theme}/icons/places/${f.place}.webp`,
          })),
        ],
      },
      {
        section: "Drives",
        items: [
          { label: "Local Disk (C:)", path: DRIVE_PATH, icon: `/themes/${theme}/icons/places/thispc.webp` },
        ],
      },
    ],
    [theme],
  );

  useEffect(() => {
    const display = displayPath(path);
    setAddress(display);
    if (windowId) {
      setTitle(windowId, `File Explorer - ${display}`);
    }

    if (path === ROOT_PATH) {
      setEntries(ROOT_ENTRIES);
      setError(null);
      return;
    }

    runtime
      .listDir(path)
      .then((res) => {
        setEntries(sortEntries(res));
        setError(null);
      })
      .catch((err) => {
        setEntries([]);
        setError(String(err));
      });
  }, [path, runtime, setTitle, windowId]);

  const navigate = (nextPath: string) => {
    const normalized = normalizePath(nextPath);
    setNav((current) => {
      const currentPath = current.history[current.index] ?? ROOT_PATH;
      if (currentPath === normalized) return current;

      const history = current.history.slice(0, current.index + 1);
      history.push(normalized);
      return { history, index: history.length - 1 };
    });
  };

  const goBack = () =>
    setNav((current) =>
      current.index > 0 ? { ...current, index: current.index - 1 } : current,
    );
  const goForward = () =>
    setNav((current) =>
      current.index < current.history.length - 1
        ? { ...current, index: current.index + 1 }
        : current,
    );

  const canGoBack = nav.index > 0;
  const canGoForward = nav.index < nav.history.length - 1;
  const parent = parentPath(path);

  return (
    <div className="explorer-shell">
      <aside className="explorer-sidebar" aria-label="Explorer shortcuts">
        {sections.map((section) => (
          <div key={section.section} className="explorer-sidebar-section">
            <div className="explorer-sidebar-title">{section.section}</div>
            {section.items.map((item) => (
              <button
                key={item.label}
                type="button"
                className={`explorer-shortcut ${normalizePath(item.path) === path ? "active" : ""}`}
                onClick={() => navigate(item.path)}
              >
                <img src={item.icon} alt="" className="explorer-shortcut-icon" draggable={false} />
                <span>{item.label}</span>
              </button>
            ))}
          </div>
        ))}
      </aside>

      <section className="explorer-main">
        <div className="explorer-toolbar">
          <button
            type="button"
            className="explorer-nav-btn"
            disabled={!canGoBack}
            title="Back"
            onClick={goBack}
          >
            Back
          </button>
          <button
            type="button"
            className="explorer-nav-btn"
            disabled={!canGoForward}
            title="Forward"
            onClick={goForward}
          >
            Forward
          </button>
          <button
            type="button"
            className="explorer-nav-btn"
            disabled={!parent}
            title="Up"
            onClick={() => parent && navigate(parent)}
          >
            Up
          </button>
          <form
            className="explorer-address"
            onSubmit={(e) => {
              e.preventDefault();
              navigate(address);
            }}
          >
            <input
              value={address}
              onChange={(e) => setAddress(e.target.value)}
              aria-label="Path"
              spellCheck={false}
            />
          </form>
        </div>

        <div className="explorer-breadcrumb">{displayPath(path)}</div>

        <div className="explorer-list" role="table" aria-label="Folder contents">
          <div className="explorer-header-row" role="row">
            <span>Name</span>
            <span>Type</span>
            <span>Size</span>
          </div>

          {error && <div className="explorer-error">Error: {error}</div>}
          {!error && entries.length === 0 && (
            <div className="explorer-empty">This folder is empty.</div>
          )}
          {!error &&
            entries.map((entry) => (
              <ExplorerRow
                key={entry.path}
                entry={entry}
                theme={theme}
                onNavigate={navigate}
                runtime={runtime}
              />
            ))}
        </div>
      </section>
    </div>
  );
}

function ExplorerRow({
  entry,
  theme,
  onNavigate,
  runtime,
}: {
  entry: DirectoryEntry;
  theme: string;
  onNavigate: (path: string) => void;
  runtime: RuntimeBridge;
}) {
  const isDir = entry.kind === "directory";
  const lowerName = entry.name.toLowerCase();
  const type = isDir
    ? entry.path === DRIVE_PATH
      ? "Drive"
      : lowerName.endsWith(".lnk")
        ? "Shortcut"
      : "File folder"
    : lowerName.endsWith(".exe")
      ? "Application"
      : lowerName.endsWith(".lnk")
        ? "Shortcut"
      : lowerName.endsWith(".txt") || lowerName.endsWith(".log")
        ? "Text document"
        : "File";

  return (
    <button
      className="explorer-row"
      type="button"
      onDoubleClick={() => {
        if (isDir) {
          onNavigate(entry.path);
        } else {
          void launchGuestPath(entry.path, runtime);
        }
      }}
    >
      <span className="explorer-name-cell">
        <img src={iconForEntry(entry, theme)} alt="" className="explorer-icon" draggable={false} />
        <span className="explorer-name">{entry.name}</span>
      </span>
      <span className="explorer-type">{type}</span>
      <span className="explorer-size">{isDir ? "" : formatSize(entry.size)}</span>
    </button>
  );
}

function normalizePath(path: string): string {
  const trimmed = path.trim();
  if (!trimmed || trimmed.toLowerCase() === "your pc") return ROOT_PATH;

  const normalized = trimmed.replace(/\//g, "\\");
  if (/^[a-z]:\\?$/i.test(normalized)) {
    return `${normalized[0].toUpperCase()}:\\`;
  }
  return normalized.replace(/\\+$/g, "");
}

function displayPath(path: string): string {
  return path ? path : "Your PC";
}

function parentPath(path: string): string | null {
  const normalized = normalizePath(path);
  if (!normalized) return null;
  if (/^[a-z]:\\$/i.test(normalized)) return ROOT_PATH;

  const idx = normalized.lastIndexOf("\\");
  if (idx <= 2) return `${normalized[0].toUpperCase()}:\\`;
  return normalized.slice(0, idx);
}

function sortEntries(entries: DirectoryEntry[]): DirectoryEntry[] {
  return [...entries].sort((a, b) => {
    if (a.kind !== b.kind) return a.kind === "directory" ? -1 : 1;
    return a.name.localeCompare(b.name, undefined, { sensitivity: "base" });
  });
}

function iconForEntry(entry: DirectoryEntry, theme: string): string {
  if (entry.path === DRIVE_PATH) return `/themes/${theme}/icons/places/thispc.webp`;

  const place = USER_FOLDERS.find((f) => f.path === entry.path);
  if (place) return `/themes/${theme}/icons/places/${place.place}.webp`;

  if (entry.kind === "directory") return `/themes/${theme}/icons/shell/folder.webp`;
  const lowerName = entry.name.toLowerCase();
  if (lowerName.endsWith(".exe") || lowerName.endsWith(".dll")) {
    return `/themes/${theme}/icons/shell/default_executable.webp`;
  }
  if (lowerName.endsWith(".txt")) return `/themes/${theme}/icons/exts/txt.webp`;
  return `/themes/${theme}/icons/exts/default.webp`;
}
