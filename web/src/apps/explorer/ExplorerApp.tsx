import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ArrowLeftFilled, ArrowRightFilled, ArrowUpFilled } from "@fluentui/react-icons";

import { useWindowStore } from "@/state/windowStore";
import { useClipboardStore } from "@/state/clipboardStore";
import { log } from "@/state/logStore";
import type { RuntimeBridge } from "@/core/bridge/runtime-bridge";
import type { DirectoryEntry } from "@/core/wasm/worker";
import { formatSize } from "@/shared/lib/utils";
import { launchGuestPath } from "@/shared/lib/guest-launch";
import { openProcessConsole } from "../process-console/ProcessConsoleApp";
import { openPeInspector } from "../pe-inspector/PeInspectorApp";
import { openProperties } from "../properties/PropertiesApp";
import { openTextReader } from "../text-reader/TextReaderApp";
import { ContextMenu, SEPARATOR, type MenuItem } from "@/modules/desktop/ContextMenu";
import { MenuBar, type MenuBarItem } from "@/shared/components/MenuBar";
import {
  copyPayloadToDir,
  createShortcut,
  decodeDragPayload,
  encodeDragPayload,
  mountFiles,
  pasteShortcut,
  performPaste,
  toPayloadEntry,
} from "@/shared/lib/clipboard";
import { resolveIcon, ICON_PLACEHOLDER } from "@/shared/lib/icons/icon-resolver";

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

type ExplorerSortKey = "name" | "date" | "type" | "size";
type ExplorerViewSize = "small" | "medium" | "large" | "mosaic";

export function openExplorer(initialPath = ROOT_PATH, runtime: RuntimeBridge) {

  const title = displayPath(normalizePath(initialPath));
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
  const clipboard = useClipboardStore();
  const surfaceRef = useRef<HTMLDivElement>(null);
  const [nav, setNav] = useState<{ history: string[]; index: number }>(() => {
    const root = normalizePath(initialPath);
    return { history: [root], index: 0 };
  });
  const path = nav.history[nav.index] ?? ROOT_PATH;
  const [address, setAddress] = useState(displayPath(path));
  const [entries, setEntries] = useState<DirectoryEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [selectedPaths, setSelectedPaths] = useState<string[]>([]);
  const [search, setSearch] = useState("");
  const [addressEditing, setAddressEditing] = useState(false);
  const [sortKey, setSortKey] = useState<ExplorerSortKey>("name");
  const [viewSize, setViewSize] = useState<ExplorerViewSize>("medium");
  const [showHidden, setShowHidden] = useState(false);
  const [hideExtensions, setHideExtensions] = useState(false);
  const [renameEntry, setRenameEntry] = useState<DirectoryEntry | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [ctx, setCtx] = useState<{
    x: number;
    y: number;
    entry: DirectoryEntry | null;
  } | null>(null);
  const [dragOver, setDragOver] = useState(false);

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
          { label: "Local Disk (C:)", path: DRIVE_PATH, icon: `/theme/icons/shell/drive_main.webp` },
        ],
      },
    ],
    [],
  );

  const commonDestinations = useMemo(
    () => [
      { label: "Desktop", path: DESKTOP_PATH },
      { label: "Documents", path: `${GUEST_HOME}\\Documents` },
      { label: "Pictures", path: `${GUEST_HOME}\\Pictures` },
      { label: "Music", path: `${GUEST_HOME}\\Music` },
      { label: "Videos", path: `${GUEST_HOME}\\Videos` },
      { label: "Local Disk (C:)", path: DRIVE_PATH },
    ],
    [],
  );

  const refreshCurrent = useCallback(() => {
    const display = displayPath(path);
    setAddress(display);
    if (windowId) {
      setTitle(windowId, display);
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

  useEffect(() => {
    refreshCurrent();
    setSelectedPaths([]);
  }, [refreshCurrent]);

  useEffect(() => {
    const handler = () => refreshCurrent();
    window.addEventListener("webwine:fs-changed", handler);
    return () => window.removeEventListener("webwine:fs-changed", handler);
  }, [refreshCurrent]);

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
  const canMutateCurrent = path !== ROOT_PATH;

  const selectedEntries = useCallback(() => {
    const selected = entries.filter((entry) => selectedPaths.includes(entry.path));
    return selected.length > 0 ? selected : [];
  }, [entries, selectedPaths]);

  const visibleEntries = useMemo(() => {
    const query = search.trim().toLowerCase();
    return sortEntries(
      entries.filter((entry) => {
        if (!showHidden && isHiddenEntry(entry)) return false;
        if (query && !entry.name.toLowerCase().includes(query)) return false;
        return true;
      }),
      sortKey,
    );
  }, [entries, search, showHidden, sortKey]);

  const selected = selectedEntries();
  const hasSelection = selected.length > 0;
  const currentFolderLabel = folderLabel(path);

  const uploadHere = useCallback(
    (destDir: string, folder: boolean) => {
      if (!runtime) return;
      const input = document.createElement("input");
      input.type = "file";
      input.multiple = true;
      if (folder) {
        input.setAttribute("webkitdirectory", "true");
      }
      input.onchange = async () => {
        const files = Array.from(input.files ?? []);
        if (files.length === 0) return;
        try {
          await mountFiles(files, destDir, runtime, {
            preserveRelativePath: folder,
          });
          refreshCurrent();
        } catch (err) {
          log("fs", `upload failed: ${err}`, "error");
        }
      };
      input.click();
    },
    [refreshCurrent, runtime],
  );

  const pasteInto = useCallback(
    async (destDir: string, shortcut = false) => {
      if (!runtime || !clipboard.has()) return;
      try {
        if (shortcut) {
          await pasteShortcut(clipboard.entries, destDir, runtime);
        } else {
          await performPaste(clipboard.entries, destDir, runtime);
          if (clipboard.isCut()) clipboard.clear();
        }
        refreshCurrent();
      } catch (err) {
        log("fs", `${shortcut ? "paste shortcut" : "paste"} failed: ${err}`, "error");
      }
    },
    [clipboard, refreshCurrent, runtime],
  );

  const copySelection = useCallback(
    (op: "copy" | "cut") => {
      const selected = selectedEntries();
      if (selected.length === 0) return;
      clipboard.setMany(selected.map(toPayloadEntry), op);
    },
    [clipboard, selectedEntries],
  );

  const createShortcutsForSelection = useCallback(async () => {
    if (!runtime) return;
    const selected = selectedEntries();
    if (selected.length === 0) return;
    try {
      for (const entry of selected) {
        await createShortcut(entry.path, entry.name, parentPath(entry.path) ?? path, runtime);
      }
      refreshCurrent();
    } catch (err) {
      log("fs", `create shortcut failed: ${err}`, "error");
    }
  }, [path, refreshCurrent, runtime, selectedEntries]);

  const createNewDirectory = useCallback(async () => {
    if (!runtime || !canMutateCurrent) return;
    setRenameEntry({ name: "", path: "", kind: "directory", size: 0 });
    setRenameValue("New folder");
  }, [canMutateCurrent, runtime]);

  const deleteSelection = useCallback(async () => {
    if (!runtime) return;
    const selected = selectedEntries();
    if (selected.length === 0) return;
    try {
      for (const entry of selected) {
        await runtime.deleteNode(entry.path);
      }
      setSelectedPaths([]);
      refreshCurrent();
    } catch (err) {
      log("fs", `delete failed: ${err}`, "error");
    }
  }, [refreshCurrent, runtime, selectedEntries]);

  const startRenameSelection = useCallback(() => {
    const selected = selectedEntries();
    if (selected.length !== 1) return;
    setRenameEntry(selected[0]);
    setRenameValue(selected[0].name);
  }, [selectedEntries]);

  const copySelectionTo = useCallback(
    async (destDir: string, move: boolean) => {
      if (!runtime) return;
      const selected = selectedEntries();
      if (selected.length === 0) return;
      try {
        await copyPayloadToDir(selected.map(toPayloadEntry), destDir, runtime, move);
        if (move) setSelectedPaths([]);
        refreshCurrent();
      } catch (err) {
        log("fs", `${move ? "move" : "copy"} failed: ${err}`, "error");
      }
    },
    [refreshCurrent, runtime, selectedEntries],
  );

  const confirmRename = useCallback(async () => {
    if (!runtime || !renameEntry || !renameValue.trim()) return;
    const name = renameValue.trim();
    try {
      if (renameEntry.path) {
        await runtime.renameNode(renameEntry.path, name);
      } else {
        await runtime.createDirectory(`${path}\\${name}`);
      }
      setRenameEntry(null);
      refreshCurrent();
    } catch (err) {
      log("fs", `rename failed: ${err}`, "error");
    }
  }, [path, refreshCurrent, renameEntry, renameValue, runtime]);

  const dropInto = useCallback(
    async (destDir: string, e: React.DragEvent) => {
      if (!runtime) return;
      const payload = decodeDragPayload(e.dataTransfer.getData("application/x-webwine-paths"));
      try {
        if (payload.length > 0) {
          await copyPayloadToDir(payload, destDir, runtime, !e.ctrlKey);
        } else if (e.dataTransfer.files.length > 0) {
          await mountFiles(Array.from(e.dataTransfer.files), destDir, runtime);
        }
        refreshCurrent();
      } catch (err) {
        log("fs", `drop failed: ${err}`, "error");
      }
    },
    [refreshCurrent, runtime],
  );

  const buildFileMenu = useCallback(
    (): MenuItem[] => [
      {
        label: "Upload file",
        disabled: !runtime || !canMutateCurrent,
        action: () => uploadHere(path, false),
      },
      {
        label: "Upload directory",
        disabled: !runtime || !canMutateCurrent,
        action: () => uploadHere(path, true),
      },
    ],
    [canMutateCurrent, path, runtime, uploadHere],
  );

  const buildDestinationMenu = useCallback(
    (move: boolean): MenuItem[] =>
      commonDestinations.map((dest) => ({
        label: dest.label,
        disabled: !hasSelection || !runtime,
        action: () => void copySelectionTo(dest.path, move),
      })),
    [commonDestinations, copySelectionTo, hasSelection, runtime],
  );

  const buildStartMenu = useCallback(
    (): MenuItem[] => [
      {
        label: "Copy",
        disabled: !hasSelection,
        action: () => copySelection("copy"),
      },
      {
        label: "Paste",
        disabled: !clipboard.has() || !canMutateCurrent,
        action: () => void pasteInto(path),
      },
      {
        label: "Cut",
        disabled: !hasSelection,
        action: () => copySelection("cut"),
      },
      {
        label: "Paste shortcut",
        disabled: !clipboard.has() || !canMutateCurrent,
        action: () => void pasteInto(path, true),
      },
      SEPARATOR,
      {
        label: "Move to",
        disabled: !hasSelection,
        children: buildDestinationMenu(true),
      },
      {
        label: "Copy to",
        disabled: !hasSelection,
        children: buildDestinationMenu(false),
      },
      SEPARATOR,
      {
        label: "Delete",
        disabled: !hasSelection,
        danger: true,
        action: () => void deleteSelection(),
      },
      {
        label: "Rename",
        disabled: selected.length !== 1,
        action: startRenameSelection,
      },
      {
        label: "Create shortcut",
        disabled: !hasSelection,
        action: () => void createShortcutsForSelection(),
      },
      SEPARATOR,
      {
        label: "New Directory",
        disabled: !canMutateCurrent,
        action: () => void createNewDirectory(),
      },
    ],
    [
      buildDestinationMenu,
      canMutateCurrent,
      clipboard,
      copySelection,
      createNewDirectory,
      createShortcutsForSelection,
      deleteSelection,
      hasSelection,
      pasteInto,
      path,
      selected.length,
      startRenameSelection,
    ],
  );

  const buildViewMenu = useCallback(
    (): MenuItem[] => [
      {
        label: "Icon size",
        children: [
          { label: "Small", checked: viewSize === "small", action: () => setViewSize("small") },
          { label: "Medium", checked: viewSize === "medium", action: () => setViewSize("medium") },
          { label: "Large", checked: viewSize === "large", action: () => setViewSize("large") },
          { label: "Mosaic", checked: viewSize === "mosaic", action: () => setViewSize("mosaic") },
        ],
      },
      {
        label: "Sort by",
        children: [
          { label: "Name", checked: sortKey === "name", action: () => setSortKey("name") },
          { label: "Date modified", checked: sortKey === "date", action: () => setSortKey("date") },
          { label: "Type", checked: sortKey === "type", action: () => setSortKey("type") },
          { label: "Size", checked: sortKey === "size", action: () => setSortKey("size") },
        ],
      },
      SEPARATOR,
      {
        label: "Hidden items",
        checked: showHidden,
        action: () => setShowHidden((value) => !value),
      },
      {
        label: "File name extensions",
        checked: !hideExtensions,
        action: () => setHideExtensions((value) => !value),
      },
    ],
    [hideExtensions, showHidden, sortKey, viewSize],
  );

  const menuBarItems = useMemo<MenuBarItem[]>(
    () => [
      { label: "File", items: buildFileMenu() },
      { label: "Start", items: buildStartMenu() },
      { label: "View", items: buildViewMenu() },
    ],
    [buildFileMenu, buildStartMenu, buildViewMenu],
  );

  const buildContextMenu = useCallback(
    (entry: DirectoryEntry | null): MenuItem[] => {
      const isFolderTarget = entry?.kind === "directory";
      const destDir = isFolderTarget ? entry.path : path;
      const canUseDest = destDir !== ROOT_PATH;
      const selected = selectedEntries();
      const menuSelection = entry && selected.some((item) => item.path === entry.path)
        ? selected
        : entry
          ? [entry]
          : [];
      const isExe = entry?.kind === "file" && entry.name.toLowerCase().endsWith(".exe");

      const items: MenuItem[] = [];

      if (entry) {
        items.push({
          label: "Open",
          action: () => {
            if (entry.kind === "directory") navigate(entry.path);
            else void launchGuestPath(entry.path, runtime);
          },
        });

        if (isExe) {
          items.push({
            label: "Run with mode...",
            children: [
              {
                label: "Keep terminal open",
                action: () => void openProcessConsole(entry.path, runtime),
              },
              {
                label: "Debug mode",
                action: () => void openProcessConsole(entry.path, runtime, { debug: true }),
              },
            ],
          });
          items.push({
            label: "Inspect",
            action: () => void openPeInspector(entry.path, runtime),
          });
        }

        if (entry.kind === "file") {
          items.push({
            label: "Edit",
            action: () => void openTextReader(entry.path, runtime),
          });
        }

        if (isFolderTarget) {
          items.push(SEPARATOR);
          items.push({
            label: "Upload file here",
            disabled: !runtime,
            action: () => uploadHere(entry.path, false),
          });
          items.push({
            label: "Upload folder here",
            disabled: !runtime,
            action: () => uploadHere(entry.path, true),
          });
        }

        items.push(SEPARATOR);
        items.push({
          label: "Cut",
          action: () => clipboard.setMany(menuSelection.map(toPayloadEntry), "cut"),
        });
        items.push({
          label: "Copy",
          action: () => clipboard.setMany(menuSelection.map(toPayloadEntry), "copy"),
        });
        items.push({
          label: "Create shortcut",
          disabled: !runtime,
          action: async () => {
            try {
              for (const item of menuSelection) {
                await createShortcut(item.path, item.name, parentPath(item.path) ?? path, runtime);
              }
              refreshCurrent();
            } catch (err) {
              log("fs", `create shortcut failed: ${err}`, "error");
            }
          },
        });
        items.push({
          label: "Delete",
          danger: true,
          action: async () => {
            try {
              for (const item of menuSelection) {
                await runtime.deleteNode(item.path);
              }
              refreshCurrent();
            } catch (err) {
              log("fs", `delete failed: ${err}`, "error");
            }
          },
        });
        items.push({
          label: "Rename",
          disabled: menuSelection.length !== 1,
          action: () => {
            setRenameEntry(menuSelection[0]);
            setRenameValue(menuSelection[0].name);
          },
        });
        items.push(SEPARATOR);
        items.push({
          label: "Properties",
          action: () => void openProperties(entry),
        });
        return items;
      }

      items.push({
        label: "Upload file here",
        disabled: !runtime || !canUseDest,
        action: () => uploadHere(destDir, false),
      });
      items.push({
        label: "Upload folder here",
        disabled: !runtime || !canUseDest,
        action: () => uploadHere(destDir, true),
      });
      items.push(SEPARATOR);
      items.push({
        label: "Paste",
        disabled: !runtime || !clipboard.has() || !canUseDest,
        action: () => void pasteInto(destDir),
      });
      items.push({
        label: "Paste shortcut",
        disabled: !runtime || !clipboard.has() || !canUseDest,
        action: () => void pasteInto(destDir, true),
      });
      items.push(SEPARATOR);
      items.push({
        label: "Refresh",
        action: refreshCurrent,
      });
      return items;
    },
    [
      clipboard,
      navigate,
      pasteInto,
      path,
      refreshCurrent,
      runtime,
      selectedEntries,
      uploadHere,
    ],
  );

  useEffect(() => {
    const onKeyDown = async (e: KeyboardEvent) => {
      if (!surfaceRef.current?.contains(document.activeElement)) return;
      if (!runtime || !(e.ctrlKey || e.metaKey)) return;
      const key = e.key.toLowerCase();
      if (!["x", "c", "v"].includes(key)) return;
      e.preventDefault();
      if (key === "x") copySelection("cut");
      if (key === "c") copySelection("copy");
      if (key === "v" && canMutateCurrent) await pasteInto(path);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [canMutateCurrent, copySelection, pasteInto, path, runtime]);

  return (
    <div
      ref={surfaceRef}
      className="h-full min-h-0 flex flex-col text-[#f2f2f2] bg-[#111111]"
      tabIndex={0}
    >
      <MenuBar items={menuBarItems} />

      <div className="flex-none flex items-center gap-1 px-2 py-1 border-b border-[var(--window-border)] bg-[#191919]">
        <NavButton label={<ArrowLeftFilled />} disabled={!canGoBack} title="Back" onClick={goBack} />
        <NavButton label={<ArrowRightFilled />} disabled={!canGoForward} title="Forward" onClick={goForward} />
        <NavButton label={<ArrowUpFilled />} disabled={!parent} title="Up" onClick={() => parent && navigate(parent)} />

        <form
          className="flex-1 min-w-[220px]"
          onSubmit={(e) => {
            e.preventDefault();
            navigate(address);
            setAddressEditing(false);
          }}
        >
          {addressEditing ? (
            <input
              className="w-full h-[30px] px-[9px] border border-[#4a4a4a] rounded-none bg-[#111111] text-[#f2f2f2] text-[12px] outline-none hover:border-[#666] focus:border-[#0078d7]"
              value={address}
              autoFocus
              onBlur={() => setAddressEditing(false)}
              onChange={(e) => setAddress(e.target.value)}
              aria-label="Path"
              spellCheck={false}
            />
          ) : (
            <button
              type="button"
              className="w-full h-[30px] px-2 border border-[#4a4a4a] rounded-none bg-[#111111] hover:border-[#666] text-left flex items-center gap-2 overflow-hidden"
              onClick={() => setAddressEditing(true)}
            >
              <img src="/theme/icons/shell/folder.webp" alt="" className="w-4 h-4 object-contain flex-none" draggable={false} />
              <Breadcrumb path={path} onNavigate={navigate} />
            </button>
          )}
        </form>

        <input
          className="w-[220px] max-[720px]:w-[140px] h-[30px] px-2 border border-[#4a4a4a] rounded-none bg-[#111111] text-[#f2f2f2] text-[12px] outline-none hover:border-[#666] focus:border-[#0078d7]"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder={`Search ${currentFolderLabel}`}
          aria-label="Search"
          spellCheck={false}
        />
      </div>

      <div className="flex-1 min-h-0 grid grid-cols-[208px_minmax(0,1fr)] max-[620px]:grid-cols-1">
        <aside
          className="min-w-0 min-h-0 py-2 px-0 overflow-y-auto border-r border-[#2b2b2b] bg-[#191919] max-[620px]:hidden"
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
          <div className="hidden">
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

          <div
            className={`flex-1 min-h-0 overflow-auto pb-2 ${dragOver ? "outline-2 outline-dashed outline-[var(--accent)] outline-offset-[-3px]" : ""}`}
            role="table"
            aria-label="Folder contents"
            onContextMenu={(e) => {
              if ((e.target as HTMLElement).closest("[data-explorer-row='true']")) return;
              e.preventDefault();
              e.stopPropagation();
              setCtx({ x: e.clientX, y: e.clientY, entry: null });
            }}
            onDragOver={(e) => {
              if (!canMutateCurrent) return;
              e.preventDefault();
              setDragOver(true);
              e.dataTransfer.dropEffect = e.ctrlKey ? "copy" : "move";
            }}
            onDragLeave={() => setDragOver(false)}
            onDrop={async (e) => {
              if (!canMutateCurrent) return;
              e.preventDefault();
              setDragOver(false);
              await dropInto(path, e);
            }}
            onClick={(e) => {
              if ((e.target as HTMLElement).closest("[data-explorer-row='true']")) return;
              setSelectedPaths([]);
            }}
          >
            {!(viewSize === "mosaic" || path === ROOT_PATH) && (
              <div
                className="sticky top-0 z-10 h-[30px] px-3 text-[#a6a6a6] border-b border-[#2b2b2b] bg-[#111111] text-[11px] grid grid-cols-[minmax(220px,1fr)_140px_132px_90px] items-center gap-x-3 max-[720px]:grid-cols-[minmax(160px,1fr)_112px_76px]"
                role="row"
              >
                <span>Name</span>
                <span className="max-[720px]:hidden">Date modified</span>
                <span>Type</span>
                <span className="text-right">Size</span>
              </div>
            )}

            {error && <div className="text-[#d13438] py-[18px] px-[14px] text-[12px]">Error: {error}</div>}
            {!error && visibleEntries.length === 0 && (
              <div className="text-[var(--shell-muted)] py-[18px] px-[14px] text-[12px]">
                {search.trim() ? "No items match your search." : "This folder is empty."}
              </div>
            )}

            {!error && path === ROOT_PATH && (
              <div className="flex flex-col p-2">
                {[
                  {
                    title: "Folders",
                    items: visibleEntries.filter((e) => USER_FOLDERS.some((f) => f.path === e.path)),
                  },
                  {
                    title: "Devices and drives",
                    items: visibleEntries.filter((e) => e.path === DRIVE_PATH || /^[a-zA-Z]:\\$/.test(e.path)),
                  },
                  {
                    title: "Other",
                    items: visibleEntries.filter(
                      (e) =>
                        !USER_FOLDERS.some((f) => f.path === e.path) &&
                        e.path !== DRIVE_PATH &&
                        !/^[a-zA-Z]:\\$/.test(e.path),
                    ),
                  },
                ]
                  .filter((group) => group.items.length > 0)
                  .map((group) => (
                    <div key={group.title} className="mb-4">
                      <div className="text-[13px] text-[#f2f2f2] font-semibold mb-2 px-1 border-b border-[rgba(255,255,255,0.1)] pb-1">
                        {group.title} ({group.items.length})
                      </div>
                      <div className="flex flex-wrap">
                        {group.items.map((entry) => (
                          <ExplorerRow
                            key={entry.path}
                            entry={entry}
                            selected={selectedPaths.includes(entry.path)}
                            selectedEntries={selectedEntries()}
                            hideExtension={hideExtensions}
                            viewSize="mosaic"
                            onNavigate={navigate}
                            runtime={runtime}
                            onSelect={(event) => {
                              setSelectedPaths((current) => {
                                if (event.ctrlKey || event.metaKey) {
                                  return current.includes(entry.path)
                                    ? current.filter((p) => p !== entry.path)
                                    : [...current, entry.path];
                                }
                                return [entry.path];
                              });
                            }}
                            onContextMenu={(event) => {
                              if (!selectedPaths.includes(entry.path)) setSelectedPaths([entry.path]);
                              setCtx({ x: event.clientX, y: event.clientY, entry });
                            }}
                            onDropInto={dropInto}
                          />
                        ))}
                      </div>
                    </div>
                  ))}
              </div>
            )}

            {!error && path !== ROOT_PATH && (
              <div className={viewSize === "mosaic" ? "flex flex-wrap p-2" : ""}>
                {visibleEntries.map((entry) => (
                  <ExplorerRow
                    key={entry.path}
                    entry={entry}
                    selected={selectedPaths.includes(entry.path)}
                    selectedEntries={selectedEntries()}
                    hideExtension={hideExtensions}
                    viewSize={viewSize}
                    onNavigate={navigate}
                    runtime={runtime}
                    onSelect={(event) => {
                      setSelectedPaths((current) => {
                        if (event.ctrlKey || event.metaKey) {
                          return current.includes(entry.path)
                            ? current.filter((path) => path !== entry.path)
                            : [...current, entry.path];
                        }
                        return [entry.path];
                      });
                    }}
                    onContextMenu={(event) => {
                      if (!selectedPaths.includes(entry.path)) {
                        setSelectedPaths([entry.path]);
                      }
                      setCtx({ x: event.clientX, y: event.clientY, entry });
                    }}
                    onDropInto={dropInto}
                  />
                ))}
              </div>
            )}
          </div>
        </section>
      </div>
      {ctx && (
        <ContextMenu
          x={ctx.x}
          y={ctx.y}
          items={buildContextMenu(ctx.entry)}
          onClose={() => setCtx(null)}
        />
      )}
      {renameEntry !== null && (
        <RenameDialog
          value={renameValue}
          onChange={setRenameValue}
          onCancel={() => setRenameEntry(null)}
          onConfirm={() => void confirmRename()}
        />
      )}
    </div>
  );
}


function NavButton({
  label,
  disabled,
  title,
  onClick,
}: {
  label: string | React.ReactNode;
  disabled?: boolean;
  title: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className="w-8 h-8 grid place-items-center rounded-none border border-transparent bg-transparent text-[#f2f2f2] text-[15px] cursor-default hover:bg-[rgba(255,255,255,0.10)] disabled:opacity-35 disabled:hover:bg-transparent"
      disabled={disabled}
      title={title}
      onClick={onClick}
    >
      {label}
    </button>
  );
}

function Breadcrumb({
  path,
  onNavigate,
}: {
  path: string;
  onNavigate: (path: string) => void;
}) {
  const parts = breadcrumbParts(path);
  return (
    <span className="min-w-0 flex items-center overflow-hidden text-[12px] text-[#f2f2f2]">
      {parts.map((part, index) => (
        <span key={`${part.path}-${index}`} className="min-w-0 flex items-center">
          {index > 0 && <span className="px-1 text-[#a6a6a6]">&gt;</span>}
          <span
            role="button"
            tabIndex={-1}
            className="px-1 h-[22px] flex items-center hover:bg-[rgba(255,255,255,0.10)] overflow-hidden text-ellipsis whitespace-nowrap"
            onMouseDown={(e) => e.stopPropagation()}
            onClick={(e) => {
              e.stopPropagation();
              onNavigate(part.path);
            }}
          >
            {part.label}
          </span>
        </span>
      ))}
    </span>
  );
}

function RenameDialog({
  value,
  onChange,
  onCancel,
  onConfirm,
}: {
  value: string;
  onChange: (value: string) => void;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  return (
    <div
      className="fixed left-1/2 top-[40%] z-[9600] w-[320px] -translate-x-1/2 -translate-y-1/2 p-3 text-[#f2f2f2] border border-[rgba(255,255,255,0.1)] rounded-none shadow-[0_16px_42px_rgba(0,0,0,0.55)]"
      style={{ background: "rgba(31,31,31,0.96)" }}
    >
      <input
        className="block w-full mb-2 bg-[#1b1b1b] border border-[#5f5f5f] rounded-none text-white outline-none focus:border-[#0078d7] px-2 py-1 text-[13px]"
        autoFocus
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") onConfirm();
          if (e.key === "Escape") onCancel();
        }}
      />
      <div className="flex gap-2 justify-end">
        <button className="bg-[#0078d7] border border-[#0078d7] rounded-none text-white px-4 py-1 text-[13px]" type="button" onClick={onConfirm}>OK</button>
        <button className="bg-[#2d2d2d] border border-[#3f3f3f] rounded-none text-white px-4 py-1 text-[13px]" type="button" onClick={onCancel}>Cancel</button>
      </div>
    </div>
  );
}


function ExplorerRow({
  entry,
  selected,
  selectedEntries,
  hideExtension,
  viewSize,
  onNavigate,
  runtime,
  onSelect,
  onContextMenu,
  onDropInto,
}: {
  entry: DirectoryEntry;
  selected: boolean;
  selectedEntries: DirectoryEntry[];
  hideExtension: boolean;
  viewSize: ExplorerViewSize;
  onNavigate: (path: string) => void;
  runtime: RuntimeBridge;
  onSelect: (event: React.MouseEvent) => void;
  onContextMenu: (event: React.MouseEvent) => void;
  onDropInto: (path: string, event: React.DragEvent) => Promise<void>;
}) {
  const [imgSrc, setImgSrc] = useState(ICON_PLACEHOLDER);

  useEffect(() => {
    resolveIcon(entry, runtime)
      .then((resolved) => setImgSrc(resolved.src))
      .catch(() => { });
  }, [entry, runtime]);

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
  const isMosaic = viewSize === "mosaic";
  const iconSize = isMosaic ? 42 : viewSize === "large" ? 30 : viewSize === "small" ? 18 : 22;
  const rowHeight = isMosaic ? 60 : viewSize === "large" ? 42 : viewSize === "small" ? 28 : 34;
  const display = hideExtension && entry.kind === "file" ? nameWithoutExtension(entry.name) : entry.name;

  return (
    <button
      className={isMosaic ? `w-[240px] m-1 px-3 py-2 flex flex-row items-center gap-4 border rounded-[var(--row-radius)] text-inherit cursor-default text-[12px] text-left hover:bg-[rgba(255,255,255,0.06)] hover:border-[rgba(255,255,255,0.09)] outline-none ${selected ? "bg-[var(--icon-selected-bg)] border-[var(--icon-selected-border)]" : "bg-transparent border-transparent"}` : `w-[calc(100%-8px)] mx-1 my-px px-2 border rounded-[var(--row-radius)] text-inherit cursor-default text-[12px] text-left grid grid-cols-[minmax(220px,1fr)_140px_132px_90px] items-center gap-x-3 max-[720px]:grid-cols-[minmax(160px,1fr)_112px_76px] hover:bg-[rgba(255,255,255,0.06)] hover:border-[rgba(255,255,255,0.09)] outline-none ${selected ? "bg-[var(--icon-selected-bg)] border-[var(--icon-selected-border)]" : "bg-transparent border-transparent"}`}
      style={{ minHeight: rowHeight }}
      type="button"
      data-explorer-row="true"
      draggable
      onClick={onSelect}
      onContextMenu={(e) => {
        e.preventDefault();
        e.stopPropagation();
        onContextMenu(e);
      }}
      onDragStart={(e) => {
        const selectedPayload = selectedEntries.some((item) => item.path === entry.path)
          ? selectedEntries
          : [entry];
        e.dataTransfer.setData(
          "application/x-webwine-paths",
          encodeDragPayload(selectedPayload.map(toPayloadEntry)),
        );
        e.dataTransfer.effectAllowed = "copyMove";
      }}
      onDragOver={(e) => {
        if (!isDir) return;
        e.preventDefault();
        e.stopPropagation();
        e.dataTransfer.dropEffect = e.ctrlKey ? "copy" : "move";
      }}
      onDrop={async (e) => {
        if (!isDir) return;
        e.preventDefault();
        e.stopPropagation();
        await onDropInto(entry.path, e);
      }}
      onDoubleClick={() => {
        if (isDir) {
          onNavigate(entry.path);
        } else {
          void launchGuestPath(entry.path, runtime);
        }
      }}
    >
      {isMosaic ? (
        <>
          <img src={imgSrc} alt="" className="object-contain flex-none" style={{ width: iconSize, height: iconSize }} draggable={false} />
          <div className="flex flex-col min-w-0">
            <span className="min-w-0 overflow-hidden text-ellipsis whitespace-nowrap">{display}</span>
            <span className="text-[var(--shell-muted)] text-[11px] min-w-0 overflow-hidden text-ellipsis whitespace-nowrap">{type}</span>
          </div>
        </>
      ) : (
        <>
          <span className="min-w-0 flex items-center gap-[9px]">
            <img src={imgSrc} alt="" className="object-contain flex-none" style={{ width: iconSize, height: iconSize }} draggable={false} />
            <span className="min-w-0 overflow-hidden text-ellipsis whitespace-nowrap">{display}</span>
          </span>
          <span className="text-[var(--shell-muted)] text-[11px] min-w-0 overflow-hidden text-ellipsis whitespace-nowrap max-[720px]:hidden"></span>
          <span className="text-[var(--shell-muted)] text-[11px] min-w-0 overflow-hidden text-ellipsis whitespace-nowrap">{type}</span>
          <span className="text-[var(--shell-muted)] text-[11px] text-right min-w-0 overflow-hidden text-ellipsis whitespace-nowrap">{isDir ? "" : formatSize(entry.size)}</span>
        </>
      )}    </button>
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

function sortEntries(entries: DirectoryEntry[], sortKey: ExplorerSortKey = "name"): DirectoryEntry[] {
  return [...entries].sort((a, b) => {
    if (a.kind !== b.kind) return a.kind === "directory" ? -1 : 1;
    if (sortKey === "size") return a.size - b.size;
    if (sortKey === "type") return entryType(a).localeCompare(entryType(b), undefined, { sensitivity: "base" });
    return a.name.localeCompare(b.name, undefined, { sensitivity: "base" });
  });
}

function entryType(entry: DirectoryEntry): string {
  const lowerName = entry.name.toLowerCase();
  if (entry.kind === "directory") {
    if (entry.path === DRIVE_PATH) return "Drive";
    return lowerName.endsWith(".lnk") ? "Shortcut" : "File folder";
  }
  if (lowerName.endsWith(".exe")) return "Application";
  if (lowerName.endsWith(".lnk")) return "Shortcut";
  if (lowerName.endsWith(".txt") || lowerName.endsWith(".log")) return "Text document";
  return "File";
}

function isHiddenEntry(entry: DirectoryEntry): boolean {
  return entry.name.startsWith(".") || entry.name.startsWith("$");
}

function nameWithoutExtension(name: string): string {
  if (name.toLowerCase().endsWith(".lnk")) return name.slice(0, -4);
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(0, dot) : name;
}

function folderLabel(path: string): string {
  if (!path) return "This PC";
  if (path === DRIVE_PATH) return "Local Disk (C:)";
  return path.split("\\").filter(Boolean).pop() ?? path;
}

function breadcrumbParts(path: string): { label: string; path: string }[] {
  if (!path) return [{ label: "This PC", path: ROOT_PATH }];
  const parts: { label: string; path: string }[] = [
    { label: "This PC", path: ROOT_PATH },
  ];
  if (/^[a-z]:\\?$/i.test(path)) {
    return [...parts, { label: "Local Disk (C:)", path: DRIVE_PATH }];
  }

  const normalized = normalizePath(path);
  parts.push({ label: "Local Disk (C:)", path: DRIVE_PATH });
  const segments = normalized.slice(3).split("\\").filter(Boolean);
  let current = DRIVE_PATH.replace(/\\$/, "");
  for (const segment of segments) {
    current = `${current}\\${segment}`;
    parts.push({ label: segment, path: current });
  }
  return parts;
}
