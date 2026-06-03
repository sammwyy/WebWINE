import { useEffect, useMemo, useRef, useState } from "react";
import { useRuntimeStore } from "../../state/runtimeStore";

import type { RuntimeBridge } from "../../core/bridge/runtime-bridge";
import type { DirectoryEntry } from "../../core/wasm/worker";
import { ICON_PLACEHOLDER, resolveIcon } from "../../shared/lib/icon-resolver";
import { launchGuestPath } from "../../shared/lib/guest-launch";

const START_MENU_PROGRAMS_ROOT =
  "C:\\Users\\guest\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs";

const DEFAULT_START_MENU_ENTRIES: DirectoryEntry[] = [
  mkLink("File Explorer.lnk", "C:\\Windows\\System32\\explorer.exe"),
  mkLink("Upload File.lnk", "C:\\Windows\\System32\\uploadfile.exe"),
  mkLink("Upload Folder.lnk", "C:\\Windows\\System32\\uploadfolder.exe"),
];

const PLACE_TILE_ENTRIES: DirectoryEntry[] = [
  mkLink("Your PC.lnk", "action:this-pc"),
  mkLink("Documents.lnk", "C:\\Users\\guest\\Documents"),
  mkLink("Pictures.lnk", "C:\\Users\\guest\\Pictures"),
  mkLink("Music.lnk", "C:\\Users\\guest\\Music"),
  mkLink("Videos.lnk", "C:\\Users\\guest\\Videos"),
];

const WIN10_LETTERS = [
  "#",
  "A",
  "B",
  "C",
  "D",
  "E",
  "F",
  "G",
  "H",
  "I",
  "J",
  "K",
  "L",
  "M",
  "N",
  "O",
  "P",
  "Q",
  "R",
  "S",
  "T",
  "U",
  "V",
  "W",
  "X",
  "Y",
  "Z",
];

interface StartMenuProps {
  onClose: () => void;
}

type StartGroup = {
  letter: string;
  entries: DirectoryEntry[];
};

export function StartMenu({ onClose }: StartMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);
  const appListRef = useRef<HTMLDivElement>(null);
  const sectionRefs = useRef<Record<string, HTMLDivElement | null>>({});

  const { runtime } = useRuntimeStore();

  const [entries, setEntries] = useState<DirectoryEntry[]>([]);
  const [letterPickerOpen, setLetterPickerOpen] = useState(false);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      const target = e.target as HTMLElement;

      if (!menuRef.current?.contains(target) && !target.closest("#start-button")) {
        onClose();
      }
    };

    document.addEventListener("click", handler);
    return () => document.removeEventListener("click", handler);
  }, [onClose]);

  useEffect(() => {
    let alive = true;

    if (!runtime) {
      setEntries([]);
      return;
    }

    const load = async () => {
      try {
        const programs = await runtime.listDir(START_MENU_PROGRAMS_ROOT);

        if (!alive) return;

        setEntries(sortEntries(programs));
      } catch {
        if (!alive) return;

        setEntries(DEFAULT_START_MENU_ENTRIES);
      }
    };

    void load();

    const refresh = () => void load();

    window.addEventListener("webwine:fs-changed", refresh);

    return () => {
      alive = false;
      window.removeEventListener("webwine:fs-changed", refresh);
    };
  }, [runtime]);

  const visibleEntries = useMemo(
    () => (entries.length > 0 ? sortEntries(entries) : DEFAULT_START_MENU_ENTRIES),
    [entries],
  );

  const groups = useMemo(() => groupEntriesByLetter(visibleEntries), [visibleEntries]);

  const availableLetters = useMemo(() => {
    const set = new Set(groups.map((group) => group.letter));
    return set;
  }, [groups]);

  const launchEntry = async (entry: DirectoryEntry) => {
    if (!runtime) return;

    await launchGuestPath(entry.path, runtime);
    onClose();
  };

  const scrollToLetter = (letter: string) => {
    const section = sectionRefs.current[letter];

    if (!section || !appListRef.current) return;

    section.scrollIntoView({
      block: "start",
      behavior: "smooth",
    });

    setLetterPickerOpen(false);
  };

  const firstAvailableLetter = groups[0]?.letter ?? "A";

  return (
    <div
      id="start-menu"
      className="fixed bottom-[40px] left-0 z-[9500] rounded-none shadow-[0_12px_32px_rgba(0,0,0,0.52)] text-[var(--menu-text)] overflow-hidden flex flex-col border-0 border-t border-r border-[rgba(255,255,255,0.10)] w-[min(648px,calc(100vw-12px))] h-[min(520px,calc(100vh-52px))] min-h-[390px] max-[560px]:w-[calc(100vw-8px)] max-[560px]:h-[min(520px,calc(100vh-48px))]"
      style={{
        background:
          "linear-gradient(90deg, rgba(0,0,0,0.38) 0 48px, transparent 48px), rgba(31,31,31,0.96)",
        backdropFilter: "blur(10px)",
        WebkitBackdropFilter: "blur(10px)",
      }}
      ref={menuRef}
      role="menu"
      onClick={(e) => e.stopPropagation()}
    >
      <div className="grid grid-cols-[48px_minmax(220px,252px)_minmax(268px,1fr)] max-[560px]:grid-cols-[48px_1fr] h-full">
        <StartRail />

        <div className="relative min-w-0 pt-3 pr-1.5 pb-2.5 pl-2 overflow-hidden">
          <div className="mb-1.5 px-2 text-[var(--shell-muted)] text-[12px] font-normal">
            All apps
          </div>

          <div
            ref={appListRef}
            className="h-[calc(100%-26px)] overflow-y-auto overflow-x-hidden pr-1 scrollbar-thin"
          >
            {groups.map((group) => (
              <div
                key={group.letter}
                ref={(node) => {
                  sectionRefs.current[group.letter] = node;
                }}
                className="scroll-mt-0"
              >
                <button
                  type="button"
                  className="w-full h-8 flex items-center px-2 bg-transparent border-none rounded-none text-[13px] text-[#d6d6d6] cursor-pointer hover:bg-[rgba(255,255,255,0.10)] active:bg-[rgba(255,255,255,0.16)] text-left"
                  onClick={() => setLetterPickerOpen(true)}
                  aria-label={`Jump from ${group.letter}`}
                >
                  {group.letter}
                </button>

                <div className="pb-1">
                  {group.entries.map((entry) => (
                    <StartMenuEntry
                      key={entry.path}
                      entry={entry}
                      runtime={runtime}
                      depth={0}
                      onLaunch={launchEntry}
                    />
                  ))}
                </div>
              </div>
            ))}
          </div>

          {letterPickerOpen && (
            <LetterPicker
              availableLetters={availableLetters}
              firstAvailableLetter={firstAvailableLetter}
              onPick={scrollToLetter}
              onClose={() => setLetterPickerOpen(false)}
            />
          )}
        </div>

        <div className="min-w-0 pt-3 px-3 pb-4 overflow-y-auto overflow-x-hidden scrollbar-thin max-[560px]:hidden">
          <div className="m-0 mb-2 px-1 text-[var(--menu-text)] text-[12px] font-normal">
            Places
          </div>

          <div className="grid grid-cols-[repeat(3,minmax(80px,1fr))] auto-rows-[90px] gap-1.5 content-start justify-start">
            {PLACE_TILE_ENTRIES.map((entry, idx) => (
              <ShortcutTile
                key={entry.path}
                entry={entry}
                runtime={runtime}
                onLaunch={launchEntry}
                index={idx}
              />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

function StartRail() {
  return (
    <div className="flex flex-col justify-between items-stretch py-1">
      <div className="flex flex-col items-stretch gap-0.5">
        <button
          className="w-12 h-12 grid place-items-center bg-transparent border-none text-[var(--menu-text)] cursor-pointer rounded-none hover:bg-[rgba(255,255,255,0.11)] active:bg-[rgba(255,255,255,0.16)]"
          type="button"
          title="WebWINE User"
          aria-label="WebWINE User"
        >
          <div className="relative w-[18px] h-[18px] border border-current rounded-full">
            <div className="absolute top-[1px] left-[3px] w-[10px] h-[6px] border border-current rounded-t-lg border-b-0" />
            <div className="absolute top-[3px] left-[5px] w-[6px] h-[6px] border border-current rounded-full" />
          </div>
        </button>
      </div>

      <div className="flex flex-col items-stretch gap-0.5">
        <button
          className="w-12 h-12 grid place-items-center bg-transparent border-none text-[var(--menu-text)] cursor-pointer rounded-none hover:bg-[rgba(255,255,255,0.11)] active:bg-[rgba(255,255,255,0.16)]"
          type="button"
          title="Power"
          aria-label="Power"
        >
          <div className="relative w-[18px] h-[18px] border border-current rounded-full">
            <div className="absolute top-[-1px] left-[7px] w-[2px] h-[9px] bg-[#1f1f1f]" />
            <div className="absolute top-[1px] left-[8px] w-[1px] h-[8px] bg-current" />
          </div>
        </button>
      </div>
    </div>
  );
}

function StartMenuEntry({
  entry,
  runtime,
  depth,
  onLaunch,
}: {
  entry: DirectoryEntry;
  runtime: RuntimeBridge | null;
  depth: number;
  onLaunch: (entry: DirectoryEntry) => Promise<void>;
}) {
  const isFolder = entry.kind === "directory";

  if (isFolder) {
    return (
      <ExpandableFolder
        entry={entry}
        runtime={runtime}
        depth={depth}
        onLaunch={onLaunch}
      />
    );
  }

  return (
    <ShortcutButton
      entry={entry}
      runtime={runtime}
      depth={depth}
      onLaunch={onLaunch}
    />
  );
}

function ExpandableFolder({
  entry,
  runtime,
  depth,
  onLaunch,
}: {
  entry: DirectoryEntry;
  runtime: RuntimeBridge | null;
  depth: number;
  onLaunch: (entry: DirectoryEntry) => Promise<void>;
}) {
  const [open, setOpen] = useState(false);
  const [children, setChildren] = useState<DirectoryEntry[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [loading, setLoading] = useState(false);
  const [iconSrc, setIconSrc] = useState(`/theme/icons/shell/folder.webp`);

  useEffect(() => {
    let alive = true;

    if (!runtime) {
      setIconSrc(`/theme/icons/shell/folder.webp`);
      return;
    }

    resolveIcon(entry, runtime)
      .then((resolved) => {
        if (!alive) return;
        setIconSrc(resolved.src);
      })
      .catch(() => {
        if (!alive) return;
        setIconSrc(`/theme/icons/shell/folder.webp`);
      });

    return () => {
      alive = false;
    };
  }, [entry, runtime]);

  const toggle = async () => {
    const nextOpen = !open;
    setOpen(nextOpen);

    if (!nextOpen || loaded || loading || !runtime) return;

    setLoading(true);

    try {
      const listed = await runtime.listDir(entry.path);
      setChildren(sortEntries(listed));
      setLoaded(true);
    } catch {
      setChildren([]);
      setLoaded(true);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div>
      <button
        className="w-full flex items-center bg-transparent border-none rounded-none text-[#f2f2f2] cursor-pointer text-[13px] gap-[10px] pr-2 py-1.5 min-h-[36px] text-left hover:bg-[rgba(255,255,255,0.11)] active:bg-[rgba(255,255,255,0.16)] focus-visible:outline focus-visible:outline-1 focus-visible:outline-white focus-visible:-outline-offset-[3px]"
        style={{ paddingLeft: 8 + depth * 16 }}
        type="button"
        role="menuitem"
        aria-expanded={open}
        onClick={() => {
          void toggle();
        }}
      >
        <span
          className={`w-3 flex-none text-[10px] text-[#d8d8d8] transition-transform ${open ? "rotate-90" : ""
            }`}
          aria-hidden="true"
        >
          ▶
        </span>

        <img
          src={iconSrc}
          alt=""
          className="flex-none w-6 h-6 object-contain"
          draggable={false}
          onError={(e) => {
            e.currentTarget.src = `/theme/icons/shell/folder.webp`;
          }}
        />

        <span className="flex-[1_1_auto] min-w-0 overflow-hidden text-ellipsis whitespace-nowrap">
          {displayLabel(entry.name)}
        </span>
      </button>

      {open && (
        <div>
          {loading && (
            <div
              className="h-8 flex items-center text-[12px] text-[var(--shell-muted)]"
              style={{ paddingLeft: 44 + depth * 16 }}
            >
              Loading...
            </div>
          )}

          {!loading && children.length === 0 && loaded && (
            <div
              className="h-8 flex items-center text-[12px] text-[var(--shell-muted)]"
              style={{ paddingLeft: 44 + depth * 16 }}
            >
              Empty
            </div>
          )}

          {!loading &&
            children.map((child) => (
              <StartMenuEntry
                key={child.path}
                entry={child}
                runtime={runtime}
                depth={depth + 1}
                onLaunch={onLaunch}
              />
            ))}
        </div>
      )}
    </div>
  );
}

function ShortcutButton({
  entry,
  runtime,
  depth,
  onLaunch,
}: {
  entry: DirectoryEntry;
  runtime: RuntimeBridge | null;
  depth: number;
  onLaunch: (entry: DirectoryEntry) => Promise<void>;
}) {
  const [iconSrc, setIconSrc] = useState(ICON_PLACEHOLDER);

  useEffect(() => {
    let alive = true;

    if (!runtime) {
      setIconSrc(`/theme/icons/shell/folder.webp`);
      return;
    }

    resolveIcon(entry, runtime)
      .then((resolved) => {
        if (!alive) return;
        setIconSrc(resolved.src);
      })
      .catch(() => {
        if (!alive) return;
        setIconSrc(`/theme/icons/shell/folder.webp`);
      });

    return () => {
      alive = false;
    };
  }, [entry, runtime]);

  return (
    <button
      className="w-full flex items-center bg-transparent border-none rounded-none text-[#f2f2f2] cursor-pointer text-[13px] gap-[10px] pr-2 py-1.5 min-h-[36px] text-left hover:bg-[rgba(255,255,255,0.11)] active:bg-[rgba(255,255,255,0.16)] focus-visible:outline focus-visible:outline-1 focus-visible:outline-white focus-visible:-outline-offset-[3px]"
      style={{ paddingLeft: 24 + depth * 16 }}
      type="button"
      role="menuitem"
      onClick={() => {
        void onLaunch(entry);
      }}
    >
      <img
        src={iconSrc}
        alt=""
        className="flex-none w-6 h-6 object-contain"
        draggable={false}
        onError={(e) => {
          e.currentTarget.src = `/theme/icons/shell/folder.webp`;
        }}
      />

      <span className="flex-[1_1_auto] min-w-0 overflow-hidden text-ellipsis whitespace-nowrap">
        {displayLabel(entry.name)}
      </span>
    </button>
  );
}

function LetterPicker({
  availableLetters,
  firstAvailableLetter,
  onPick,
  onClose,
}: {
  availableLetters: Set<string>;
  firstAvailableLetter: string;
  onPick: (letter: string) => void;
  onClose: () => void;
}) {
  return (
    <div
      className="absolute inset-x-0 top-0 bottom-0 z-20 bg-[rgba(31,31,31,0.98)] border-r border-[rgba(255,255,255,0.08)] shadow-[8px_0_24px_rgba(0,0,0,0.28)]"
      role="dialog"
      aria-label="Choose letter"
    >
      <div className="h-full flex flex-col">
        <div className="h-10 px-3 flex items-center justify-between text-[12px] text-[var(--shell-muted)]">
          <span>All apps</span>

          <button
            type="button"
            className="w-8 h-8 grid place-items-center bg-transparent border-none text-[#f2f2f2] cursor-pointer hover:bg-[rgba(255,255,255,0.11)] active:bg-[rgba(255,255,255,0.16)]"
            onClick={onClose}
            aria-label="Close letter picker"
          >
            ×
          </button>
        </div>

        <div className="grid grid-cols-4 gap-1 px-2 pb-2">
          {WIN10_LETTERS.map((letter) => {
            const enabled = availableLetters.has(letter);
            const fallbackTarget = enabled ? letter : firstAvailableLetter;

            return (
              <button
                key={letter}
                type="button"
                disabled={!enabled && !firstAvailableLetter}
                className={[
                  "h-10 rounded-none border-none text-[15px] font-normal",
                  enabled
                    ? "bg-transparent text-[#f2f2f2] cursor-pointer hover:bg-[rgba(255,255,255,0.11)] active:bg-[rgba(255,255,255,0.16)]"
                    : "bg-transparent text-[rgba(255,255,255,0.28)] cursor-default",
                ].join(" ")}
                onClick={() => {
                  if (enabled) {
                    onPick(letter);
                    return;
                  }

                  onPick(fallbackTarget);
                }}
              >
                {letter}
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}

function ShortcutTile({
  entry,
  runtime,
  onLaunch,
  index,
}: {
  entry: DirectoryEntry;
  runtime: RuntimeBridge | null;
  onLaunch: (entry: DirectoryEntry) => Promise<void>;
  index?: number;
}) {
  const [iconSrc, setIconSrc] = useState(ICON_PLACEHOLDER);

  useEffect(() => {
    let alive = true;

    if (!runtime) {
      setIconSrc(`/theme/icons/shell/folder.webp`);
      return;
    }

    resolveIcon(entry, runtime)
      .then((resolved) => {
        if (!alive) return;
        setIconSrc(resolved.src);
      })
      .catch(() => {
        if (!alive) return;
        setIconSrc(`/theme/icons/shell/folder.webp`);
      });

    return () => {
      alive = false;
    };
  }, [entry, runtime]);

  const bgClass =
    index !== undefined && index % 2 !== 0
      ? "bg-[#107c10] hover:bg-[#168f16]"
      : "bg-[#0078d7] hover:bg-[#1683dc]";

  return (
    <button
      className={`relative flex flex-col justify-between items-start border-2 border-transparent rounded-none text-white cursor-pointer overflow-hidden pt-2.5 px-[9px] pb-2 text-left focus-visible:outline focus-visible:outline-1 focus-visible:outline-white focus-visible:-outline-offset-[3px] ${bgClass}`}
      type="button"
      onClick={() => {
        void onLaunch(entry);
      }}
    >
      <img
        src={iconSrc}
        alt=""
        className="w-[34px] h-[34px] object-contain flex-none drop-shadow-[0_1px_1px_rgba(0,0,0,0.32)]"
        draggable={false}
        onError={(e) => {
          e.currentTarget.src = `/theme/icons/shell/folder.webp`;
        }}
      />

      <span className="block max-w-full text-[12px] leading-[1.2] break-words text-white [text-shadow:0_1px_1px_rgba(0,0,0,0.24)]">
        {displayLabel(entry.name)}
      </span>
    </button>
  );
}

function groupEntriesByLetter(entries: DirectoryEntry[]): StartGroup[] {
  const grouped = new Map<string, DirectoryEntry[]>();

  for (const entry of sortEntries(entries)) {
    const letter = getStartLetter(displayLabel(entry.name));

    if (!grouped.has(letter)) {
      grouped.set(letter, []);
    }

    grouped.get(letter)!.push(entry);
  }

  return Array.from(grouped.entries())
    .sort(([a], [b]) => {
      if (a === "#") return -1;
      if (b === "#") return 1;

      return a.localeCompare(b, undefined, { sensitivity: "base" });
    })
    .map(([letter, groupedEntries]) => ({
      letter,
      entries: groupedEntries,
    }));
}

function getStartLetter(label: string): string {
  const normalized = label.trim();

  if (!normalized) return "#";

  const first = normalized[0].toUpperCase();

  if (first >= "A" && first <= "Z") {
    return first;
  }

  return "#";
}

function displayLabel(name: string): string {
  return name.toLowerCase().endsWith(".lnk") ? name.slice(0, -4) : name;
}

function mkLink(name: string, target: string): DirectoryEntry {
  return {
    name,
    path: `${START_MENU_PROGRAMS_ROOT}\\${name}`,
    kind: "file",
    size: target.length,
  };
}

function sortEntries(entries: DirectoryEntry[]): DirectoryEntry[] {
  return [...entries].sort((a, b) => {
    const aIsFolder = a.kind === "directory";
    const bIsFolder = b.kind === "directory";

    if (aIsFolder !== bIsFolder) {
      return aIsFolder ? -1 : 1;
    }

    return displayLabel(a.name).localeCompare(displayLabel(b.name), undefined, {
      sensitivity: "base",
      numeric: true,
    });
  });
}