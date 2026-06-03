import { useEffect, useMemo, useState } from "react";
import { useWindowStore } from "../../state/windowStore";
import type { RuntimeBridge } from "../../core/bridge/runtime-bridge";
import type { DirectoryEntry } from "../../core/wasm/worker";
import { formatSize } from "../../shared/lib/utils";

import { launchGuestPath } from "../../shared/lib/guest-launch";

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

  const title = initialPath ? `File Explorer - ${initialPath}` : "File Explorer - Your PC";
  const id = useWindowStore.getState().openWindow({
    title,
    icon: `/theme/icons/apps/explorer.webp`,
    width: 920,
    height: 560,
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
          { label: "Your PC", path: ROOT_PATH, icon: `/theme/icons/places/thispc.webp` },
          ...USER_FOLDERS.map((f) => ({
            label: f.label,
            path: f.path,
            icon: `/theme/icons/places/${f.place}.webp`,
          })),
        ],
      },
      {
        section: "Drives",
        items: [
          { label: "Local Disk (C:)", path: DRIVE_PATH, icon: `/theme/icons/places/thispc.webp` },
        ],
      },
    ],
    [],
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
    <div className="grid grid-cols-[188px_minmax(0,1fr)] max-[560px]:grid-cols-1 h-full min-h-0 text-[#f2f2f2] bg-[#111111]">
      <aside
        className="min-w-0 py-2 px-0 overflow-y-auto border-r border-[#2b2b2b] bg-[#191919] max-[560px]:hidden"
        aria-label="Explorer shortcuts">
        {sections.map((section) => (
          <div key={section.section} className="mt-3 first:mt-0">
            <div className="px-3 pb-1.5 text-[#a6a6a6] text-[11px] font-normal">
              {section.section}
            </div>
            {section.items.map((item) => {
              const isActive = normalizePath(item.path) === path;
              return (
                <button
                  key={item.label}
                  type="button"
                  className={[
                    "w-full min-h-[32px] flex items-center gap-[9px]",
                    "py-[5px] px-3 rounded-none cursor-default",
                    "text-[12px] text-left text-[#f2f2f2]",
                    "border border-transparent",
                    "hover:bg-[rgba(255,255,255,0.09)]",
                    isActive
                      ? "bg-[rgba(255,255,255,0.13)] border-[rgba(255,255,255,0.08)]"
                      : "bg-transparent",
                  ].join(" ")}
                  onClick={() => navigate(item.path)}>
                  <img src={item.icon} alt="" className="w-5 h-5 object-contain flex-none" draggable={false} />
                  <span>{item.label}</span>
                </button>
              );
            })}
          </div>
        ))}
      </aside>

      <section className="min-w-0 min-h-0 flex flex-col bg-[var(--window-bg)]">
        <div className="flex items-center gap-1 px-2 py-[6px] border-b border-[var(--window-border)] bg-[#191919]">
          <button
            type="button"
            className="w-8 h-8 grid place-items-center rounded-none border border-transparent bg-transparent text-[#f2f2f2] text-[18px] cursor-default hover:bg-[rgba(255,255,255,0.10)] disabled:opacity-35 disabled:hover:bg-transparent"
            disabled={!canGoBack}
            title="Back"
            onClick={goBack}
          >
            ‹
          </button>

          <button
            type="button"
            className="w-8 h-8 grid place-items-center rounded-none border border-transparent bg-transparent text-[#f2f2f2] text-[18px] cursor-default hover:bg-[rgba(255,255,255,0.10)] disabled:opacity-35 disabled:hover:bg-transparent"
            disabled={!canGoForward}
            title="Forward"
            onClick={goForward}
          >
            ›
          </button>

          <button
            type="button"
            className="w-8 h-8 grid place-items-center rounded-none border border-transparent bg-transparent text-[#f2f2f2] text-[15px] cursor-default hover:bg-[rgba(255,255,255,0.10)] disabled:opacity-35 disabled:hover:bg-transparent"
            disabled={!parent}
            title="Up"
            onClick={() => parent && navigate(parent)}
          >
            ↑
          </button>

          <form
            className="flex-1 min-w-0 ml-1"
            onSubmit={(e) => {
              e.preventDefault();
              navigate(address);
            }}
          >
            <input
              className="w-full h-[30px] px-[9px] border border-[#4a4a4a] rounded-none bg-[#111111] text-[#f2f2f2] text-[12px] outline-none hover:border-[#666] focus:border-[#0078d7]"
              value={address}
              onChange={(e) => setAddress(e.target.value)}
              aria-label="Path"
              spellCheck={false}
            />
          </form>
        </div>

        <div className="flex-1 min-h-0 overflow-auto pb-2" role="table" aria-label="Folder contents">
          <div
            className="sticky top-0 z-10 h-[28px] px-3 text-[#a6a6a6] border-b border-[#2b2b2b] bg-[#111111] text-[11px] grid grid-cols-[minmax(180px,1fr)_140px_90px] items-center gap-x-3 max-[560px]:grid-cols-[minmax(150px,1fr)_82px]"
            role="row"
          >
            <span>Name</span>
            <span>Type</span>
            <span className="max-[560px]:hidden text-right">Size</span>
          </div>

          {error && <div className="text-[#d13438] py-[18px] px-[14px] text-[12px]">Error: {error}</div>}
          {!error && entries.length === 0 && (
            <div className="text-[var(--shell-muted)] py-[18px] px-[14px] text-[12px]">This folder is empty.</div>
          )}
          {!error &&
            entries.map((entry) => (
              <ExplorerRow
                key={entry.path}
                entry={entry}

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
  onNavigate,
  runtime,
}: {
  entry: DirectoryEntry;

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
      className={`w-[calc(100%-8px)] min-h-[34px] mx-1 my-px px-2 border border-transparent rounded-[var(--row-radius)] bg-transparent text-inherit cursor-default text-[12px] text-left grid grid-cols-[minmax(180px,1fr)_126px_86px] items-center gap-x-3 max-[560px]:grid-cols-[minmax(150px,1fr)_82px] hover:bg-[rgba(255,255,255,0.06)] hover:border-[rgba(255,255,255,0.09)] focus:bg-[var(--icon-selected-bg)] focus:border-[var(--icon-selected-border)] outline-none`}
      type="button"
      onDoubleClick={() => {
        if (isDir) {
          onNavigate(entry.path);
        } else {
          void launchGuestPath(entry.path, runtime);
        }
      }}
    >
      <span className="min-w-0 flex items-center gap-[9px]">
        <img src={iconForEntry(entry)} alt="" className="w-[22px] h-[22px] object-contain flex-none" draggable={false} />
        <span className="min-w-0 overflow-hidden text-ellipsis whitespace-nowrap">{entry.name}</span>
      </span>
      <span className="text-[var(--shell-muted)] text-[11px] min-w-0 overflow-hidden text-ellipsis whitespace-nowrap">{type}</span>
      <span className="text-[var(--shell-muted)] text-[11px] text-right min-w-0 overflow-hidden text-ellipsis whitespace-nowrap max-[560px]:hidden">{isDir ? "" : formatSize(entry.size)}</span>
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

function iconForEntry(entry: DirectoryEntry): string {
  if (entry.path === DRIVE_PATH) return `/theme/icons/places/thispc.webp`;

  const place = USER_FOLDERS.find((f) => f.path === entry.path);
  if (place) return `/theme/icons/places/${place.place}.webp`;

  if (entry.kind === "directory") return `/theme/icons/shell/folder.webp`;
  const lowerName = entry.name.toLowerCase();
  if (lowerName.endsWith(".exe") || lowerName.endsWith(".dll")) {
    return `/theme/icons/shell/default_executable.webp`;
  }
  if (lowerName.endsWith(".txt")) return `/theme/icons/exts/txt.webp`;
  return `/theme/icons/exts/default.webp`;
}
