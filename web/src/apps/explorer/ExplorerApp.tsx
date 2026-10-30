import { useCallback, useEffect, useMemo, useRef, useState } from "react";

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
import { showPrompt } from "../prompt/PromptDialog";
import {
  AddressBar,
  Button,
  Command,
  CommandBar,
  CommandSpacer,
  DetailsPane,
  DialogFooter,
  Input,
  NavButtons,
  NavLink,
  NavPane,
  NavSection,
  SearchBox,
  Select,
} from "@/shared/ui";
import { SystemIcon } from "@/shared/icons";
import { GlassPanel, WindowBody } from "@/modules/windows";
import { iconUrl, useIconPack } from "@/shared/lib/personalization";

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
    icon: iconUrl("apps/explorer"),
    width: 920,
    height: 560,
    minWidth: 420,
    minHeight: 280,
    extendGlass: true,
    content: <div />,
  });

  useWindowStore.getState().setContent(
    id,
    <ExplorerApp initialPath={initialPath} runtime={runtime} windowId={id} />,
  );
}

/** When set, Explorer runs as a modal file picker: double-clicking / "Open" a
 * file resolves `onPick(path)` instead of launching it, and a footer with a
 * filename box + Open/Save/Cancel is shown. */
export interface FilePickerProps {
  save: boolean;
  filter: string;
  defaultName: string;
  onPick: (path: string | null) => void;
}

export function ExplorerApp({
  initialPath,
  runtime,
  windowId,
  picker,
}: {
  initialPath: string;
  runtime: RuntimeBridge;
  windowId: string;
  picker?: FilePickerProps;
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
  const [ctx, setCtx] = useState<{
    x: number;
    y: number;
    entry: DirectoryEntry | null;
    /** explicit items (command bar drop-downs); otherwise the context menu for `entry` */
    items?: MenuItem[];
  } | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const [pickerName, setPickerName] = useState(picker?.defaultName ?? "");
  const pickerFilters = useMemo(() => parsePickerFilter(picker?.filter ?? ""), [picker?.filter]);
  const [pickerFilterIdx, setPickerFilterIdx] = useState(0);

  // In picker mode, choosing a file resolves the dialog (instead of launching).
  const onOpenFile = picker ? (entry: DirectoryEntry) => picker.onPick(entry.path) : undefined;
  const submitPicker = () => {
    const n = pickerName.trim();
    if (!n) return;
    picker?.onPick(/^[a-z]:\\/i.test(n) ? n : `${path.replace(/\\+$/g, "")}\\${n}`);
  };

  const iconPack = useIconPack();
  const sections = useMemo<SidebarSection[]>(
    () => [
      {
        section: "Quick access",
        items: [
          { label: "Your PC", path: ROOT_PATH, icon: iconUrl("places/thispc") },
          ...USER_FOLDERS.map((f) => ({
            label: f.label,
            path: f.path,
            icon: iconUrl(`places/${f.place}`),
          })),
        ],
      },
      {
        section: "Drives",
        items: [
          { label: "Local Disk (C:)", path: DRIVE_PATH, icon: iconUrl("shell/drive_main") },
        ],
      },
    ],
    // recompute the icon URLs when the icon pack changes
    [iconPack],
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

  /** Rename entry, or create a folder in the current directory when it has no path. */
  const promptName = useCallback(
    async (entry: DirectoryEntry, initial: string) => {
      const name = await showPrompt({
        title: entry.path ? "Rename" : "New Folder",
        label: entry.path ? "New name:" : "Folder name:",
        value: initial,
      });
      if (!runtime || !name) return;
      try {
        if (entry.path) await runtime.renameNode(entry.path, name);
        else await runtime.createDirectory(`\\${name}`);
        refreshCurrent();
      } catch (err) {
        log("fs", `rename failed: ${err}`, "error");
      }
    },
    [path, refreshCurrent, runtime],
  );

  const createNewDirectory = useCallback(async () => {
    if (!runtime || !canMutateCurrent) return;
    void promptName({ name: "", path: "", kind: "directory", size: 0 }, "New folder");
  }, [canMutateCurrent, runtime, promptName]);

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
    void promptName(selected[0], selected[0].name);
  }, [selectedEntries, promptName]);

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
            void promptName(menuSelection[0], menuSelection[0].name);
          },
        });
        items.push(SEPARATOR);
        items.push({
          label: "Properties",
          action: () => void openProperties(entry, runtime!),
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

  const openCommandMenu = (e: React.MouseEvent<HTMLElement>, items: MenuItem[]) => {
    const rect = e.currentTarget.getBoundingClientRect();
    setCtx({ x: rect.left, y: rect.bottom, entry: null, items });
  };

  const details = selected.length === 1 ? selected[0] : null;
  const showHeader = !(viewSize === "mosaic" || path === ROOT_PATH);
  const pickerEntries = picker
    ? visibleEntries.filter(
        (e) => e.kind === "directory" || matchesPickerFilter(e.name, pickerFilters[pickerFilterIdx]?.pattern ?? "*.*"),
      )
    : visibleEntries;

  const rowProps = (entry: DirectoryEntry) => ({
    entry,
    selected: selectedPaths.includes(entry.path),
    selectedEntries: selected,
    hideExtension: hideExtensions,
    onNavigate: navigate,
    runtime,
    onOpenFile,
    onDropInto: dropInto,
    onSelect: (event: React.MouseEvent) => {
      if (picker && entry.kind === "file") setPickerName(entry.name);
      setSelectedPaths((current) => {
        if (event.ctrlKey || event.metaKey) {
          return current.includes(entry.path) ? current.filter((p) => p !== entry.path) : [...current, entry.path];
        }
        return [entry.path];
      });
    },
    onContextMenu: (event: React.MouseEvent) => {
      if (!selectedPaths.includes(entry.path)) setSelectedPaths([entry.path]);
      setCtx({ x: event.clientX, y: event.clientY, entry });
    },
  });

  const sortCol = (key: ExplorerSortKey, label: string, className = "") => (
    <div
      className={`ww-listview-col ${sortKey === key ? "sorted" : ""} ${className}`}
      onClick={() => setSortKey(key)}
    >
      {label}
    </div>
  );

  return (
    <div ref={surfaceRef} className="flex flex-col flex-1 min-h-0 outline-none" tabIndex={-1}>
      <GlassPanel>
        <NavButtons
          canGoBack={canGoBack}
          canGoForward={canGoForward}
          canGoUp={!!parent}
          onBack={goBack}
          onForward={goForward}
          onUp={() => parent !== null && navigate(parent)}
        />
        <AddressBar
          icon={iconUrl("places/thispc")}
          path={breadcrumbParts(path).map((part) => ({
            label: part.label,
            icon: iconUrl(part.path === ROOT_PATH ? "places/thispc" : part.path === DRIVE_PATH ? "shell/drive_main" : "shell/folder"),
            onClick: () => navigate(part.path),
          }))}
          editValue={displayPath(path)}
          onNavigate={navigate}
          onRefresh={refreshCurrent}
        />
        <SearchBox
          className="w-[200px] shrink-0 max-[720px]:w-[130px]"
          placeholder={`Search ${currentFolderLabel}`}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
      </GlassPanel>

      <WindowBody>
        <CommandBar>
          <Command dropdown onClick={(e) => openCommandMenu(e, buildStartMenu())}>
            Organize
          </Command>
          <Command dropdown icon={<SystemIcon name="view-list-details" />} onClick={(e) => openCommandMenu(e, buildViewMenu())}>
            Views
          </Command>
          {canMutateCurrent && (
            <>
              <Command icon={iconUrl("shell/folder")} onClick={() => void createNewDirectory()}>
                New folder
              </Command>
              <Command
                icon={iconUrl("exts/default")}
                dropdown
                onClick={(e) =>
                  openCommandMenu(e, [
                    { label: "Upload files…", action: () => uploadHere(path, false) },
                    { label: "Upload folder…", action: () => uploadHere(path, true) },
                  ])
                }
              >
                Upload
              </Command>
            </>
          )}
          {details && (
            <Command icon={<SystemIcon name="document-properties" />} onClick={() => void openProperties(details, runtime)}>
              Properties
            </Command>
          )}
          <CommandSpacer />
          <Command title="Refresh" icon={<SystemIcon name="view-refresh" />} onClick={refreshCurrent} />
        </CommandBar>

        <div className="flex flex-1 min-h-0">
          <NavPane width={196} className="max-[620px]:hidden" footer={picker ? undefined : "Folders"}>
            {sections.map((section) => (
              <NavSection key={section.section} title={section.section}>
                {section.items.map((item) => (
                  <NavLink
                    key={item.label}
                    icon={item.icon}
                    selected={normalizePath(item.path) === path}
                    onClick={() => navigate(item.path)}
                  >
                    {item.label}
                  </NavLink>
                ))}
              </NavSection>
            ))}
          </NavPane>

          <div
            className={`ww-listview ${dragOver ? "outline-2 outline-dashed outline-[var(--ww-accent)] outline-offset-[-3px]" : ""}`}
            tabIndex={0}
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
            {showHeader && (
              <div className="ww-listview-header explorer-cols" role="row">
                {sortCol("name", "Name")}
                {sortCol("date", "Date modified", "max-[720px]:hidden")}
                {sortCol("type", "Type")}
                {sortCol("size", "Size", "justify-end")}
              </div>
            )}

            <div className="ww-listview-body">
              {error && <div className="px-3 py-4 text-[var(--ww-danger)]">Error: {error}</div>}
              {!error && pickerEntries.length === 0 && (
                <div className="px-3 py-4 ww-muted">
                  {search.trim() ? "No items match your search." : "This folder is empty."}
                </div>
              )}

              {!error && path === ROOT_PATH &&
                [
                  { title: "Folders", items: visibleEntries.filter((e) => USER_FOLDERS.some((f) => f.path === e.path)) },
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
                    <div key={group.title}>
                      <div className="ww-listview-group">
                        <span>
                          {group.title} ({group.items.length})
                        </span>
                      </div>
                      <div className="ww-listview-tiles">
                        {group.items.map((entry) => (
                          <ExplorerRow key={entry.path} {...rowProps(entry)} viewSize="mosaic" />
                        ))}
                      </div>
                    </div>
                  ))}

              {!error && path !== ROOT_PATH && (
                <div className={viewSize === "mosaic" ? "ww-listview-tiles" : ""}>
                  {pickerEntries.map((entry) => (
                    <ExplorerRow key={entry.path} {...rowProps(entry)} viewSize={viewSize} />
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>

        {!picker && (
          <DetailsPane
            icon={details ? <ExplorerEntryIcon entry={details} runtime={runtime} /> : iconUrl(path === ROOT_PATH ? "places/thispc" : "shell/folder")}
            title={details ? details.name : currentFolderLabel}
            properties={
              details
                ? [
                    ["Type", entryType(details)],
                    ...(details.kind === "file" ? [["Size", formatSize(details.size)] as [string, string]] : []),
                    ["Location", parentPath(details.path) ? displayPath(parentPath(details.path)!) : "This PC"],
                  ]
                : [
                    ["Items", String(visibleEntries.length)],
                    ...(selected.length > 1 ? [["Selected", `${selected.length} items`] as [string, string]] : []),
                  ]
            }
          />
        )}

        {picker && (
          <DialogFooter className="flex-col !items-stretch">
            <div className="flex items-center gap-2">
              <span className="w-16">File name:</span>
              <Input
                className="flex-1"
                value={pickerName}
                autoFocus
                onChange={(e) => setPickerName(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && submitPicker()}
              />
              {pickerFilters.length > 0 && (
                <Select
                  className="max-w-[180px]"
                  value={pickerFilterIdx}
                  onChange={(e) => setPickerFilterIdx(Number(e.target.value))}
                >
                  {pickerFilters.map((f, i) => (
                    <option key={i} value={i}>
                      {f.label}
                    </option>
                  ))}
                </Select>
              )}
            </div>
            <div className="flex justify-end gap-2">
              <Button isDefault onClick={submitPicker}>
                {picker.save ? "Save" : "Open"}
              </Button>
              <Button onClick={() => picker.onPick(null)}>Cancel</Button>
            </div>
          </DialogFooter>
        )}
      </WindowBody>

      {ctx && (
        <ContextMenu
          x={ctx.x}
          y={ctx.y}
          items={ctx.items ?? buildContextMenu(ctx.entry)}
          onClose={() => setCtx(null)}
        />
      )}
    </div>
  );
}

/** Large icon of an entry (details pane). */
function ExplorerEntryIcon({ entry, runtime }: { entry: DirectoryEntry; runtime: RuntimeBridge }) {
  const [src, setSrc] = useState(ICON_PLACEHOLDER);
  useEffect(() => {
    resolveIcon(entry, runtime)
      .then((resolved) => setSrc(resolved.src))
      .catch(() => {});
  }, [entry, runtime]);
  return <img src={src} alt="" draggable={false} />;
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
  onOpenFile,
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
  onOpenFile?: (entry: DirectoryEntry) => void;
}) {
  const [imgSrc, setImgSrc] = useState(ICON_PLACEHOLDER);

  useEffect(() => {
    resolveIcon(entry, runtime)
      .then((resolved) => setImgSrc(resolved.src))
      .catch(() => { });
  }, [entry, runtime]);

  const isDir = entry.kind === "directory";
  const type = entryType(entry);
  const isMosaic = viewSize === "mosaic";
  const iconSize = isMosaic ? 40 : viewSize === "large" ? 32 : viewSize === "small" ? 16 : 20;
  const display = hideExtension && entry.kind === "file" ? nameWithoutExtension(entry.name) : entry.name;

  return (
    <div
      className={`${isMosaic ? "ww-listview-tile" : "ww-listview-row explorer-cols"} ${selected ? "selected" : ""}`}
      style={isMosaic ? undefined : { height: "auto", minHeight: iconSize + 6 }}
      role="row"
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
        } else if (onOpenFile) {
          onOpenFile(entry);
        } else {
          void launchGuestPath(entry.path, runtime);
        }
      }}
    >
      {isMosaic ? (
        <>
          <img src={imgSrc || undefined} alt="" className="object-contain flex-none" style={{ width: iconSize, height: iconSize }} draggable={false} />
          <div className="ww-tile-text">
            <span className="truncate">{display}</span>
            <span className="muted truncate">{type}</span>
            {!isDir && <span className="muted truncate">{formatSize(entry.size)}</span>}
          </div>
        </>
      ) : (
        <>
          <span className="ww-listview-cell flex items-center gap-[7px]">
            <img src={imgSrc || undefined} alt="" className="object-contain flex-none" style={{ width: iconSize, height: iconSize }} draggable={false} />
            <span className="truncate">{display}</span>
          </span>
          <span className="ww-listview-cell muted max-[720px]:hidden"></span>
          <span className="ww-listview-cell muted">{type}</span>
          <span className="ww-listview-cell muted text-right">{isDir ? "" : formatSize(entry.size)}</span>
        </>
      )}
    </div>
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

// file-picker filter helpers
function parsePickerFilter(filter: string): { label: string; pattern: string }[] {
  if (!filter) return [];
  const parts = filter.split("|");
  const out: { label: string; pattern: string }[] = [];
  for (let i = 0; i + 1 < parts.length; i += 2) out.push({ label: parts[i], pattern: parts[i + 1] });
  return out;
}

function matchesPickerFilter(name: string, pattern: string): boolean {
  return pattern.split(";").some((p) => {
    p = p.trim();
    if (!p || p === "*.*" || p === "*") return true;
    const re = new RegExp(
      "^" + p.replace(/[.+^${}()|[\]\\]/g, "\\$&").replace(/\*/g, ".*").replace(/\?/g, ".") + "$",
      "i",
    );
    return re.test(name);
  });
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
