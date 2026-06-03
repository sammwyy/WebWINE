import { useEffect, useMemo, useRef, useState } from "react";
import { useRuntimeStore } from "../../stores/useRuntimeStore.js";
import { useThemeStore } from "../../stores/useThemeStore.js";
import type { RuntimeBridge } from "../../lib/runtime-bridge.js";
import type { DirectoryEntry } from "../../lib/worker.js";
import { ICON_PLACEHOLDER, resolveIcon } from "../../lib/icon-resolver.js";
import { launchGuestPath } from "../../lib/guest-launch.js";
import styles from "./StartMenu.module.css";

const START_MENU_PROGRAMS_ROOT =
  "C:\\Users\\guest\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs";

const DEFAULT_START_MENU_ENTRIES: DirectoryEntry[] = [
  mkLink("File Explorer.lnk", "C:\\Windows\\System32\\explorer.exe"),
  mkLink("Themes.lnk", "C:\\Windows\\System32\\themes.exe"),
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

interface StartMenuProps {
  onClose: () => void;
}

export function StartMenu({ onClose }: StartMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);
  const { theme, getEffectiveStartMenuLayout } = useThemeStore();
  const { runtime } = useRuntimeStore();
  const layout = getEffectiveStartMenuLayout();
  const [entries, setEntries] = useState<DirectoryEntry[]>([]);

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

  const launchEntry = async (entry: DirectoryEntry) => {
    if (!runtime) return;
    await launchGuestPath(entry.path, runtime);
    onClose();
  };

  const renderEntry = (entry: DirectoryEntry) => (
    <ShortcutButton
      key={entry.path}
      entry={entry}
      runtime={runtime}
      theme={theme}
      onLaunch={launchEntry}
    />
  );

  const renderClassic = () => (
    <div className="start-menu-layout classic">
      <div className={`${styles["start-menu-sidebar"]} start-menu-sidebar`}>
        <span className={`${styles["start-menu-sidebar-text"]} start-menu-sidebar-text`}>WebWINE</span>
      </div>
      <div className={`${styles["start-menu-content"]} start-menu-content classic-menu-content`}>
        {visibleEntries.map(renderEntry)}
      </div>
    </div>
  );

  const renderExperience = () => (
    <div className="start-menu-layout experience">
      <div className={`${styles["start-menu-header"]} start-menu-header`}>
        <img
          src={`/themes/${theme}/icons/shell/msg_inform.webp`}
          className={`${styles["user-avatar"]} user-avatar`}
          alt="User"
          onError={(e) => {
            e.currentTarget.style.display = "none";
          }}
        />
        <span className={`${styles["user-name"]} user-name`}>WebWINE User</span>
      </div>
      <div className={`${styles["start-menu-content"]} start-menu-content`}>
        <div className={`${styles["shell-menu-title"]} shell-menu-title`}>Programs</div>
        {visibleEntries.map(renderEntry)}
      </div>
    </div>
  );

  const renderFluent = () => (
    <div className="start-menu-layout fluent">
      <div className={`${styles["start-menu-rail"]} start-menu-rail`}>
        <div className={`${styles["rail-top"]} rail-top`}>
          <button
            className={`${styles["rail-btn"]} rail-btn rail-btn-user`}
            type="button"
            title="WebWINE User"
            aria-label="WebWINE User"
          >
            <span className={`${styles["rail-icon"]} rail-icon`} aria-hidden="true" />
          </button>
        </div>
      </div>
      <div className={`${styles["start-menu-apps"]} start-menu-apps`}>
        <div className={`${styles["start-menu-section-title"]} start-menu-section-title`}>Programs</div>
        {visibleEntries.map(renderEntry)}
      </div>
      <div className={`${styles["start-menu-tiles"]} start-menu-tiles`}>
        <div className={`${styles["tile-group-title"]} tile-group-title`}>Places</div>
        <div className={`${styles["tile-grid"]} tile-grid`}>
          {PLACE_TILE_ENTRIES.map((entry) => (
            <ShortcutTile
              key={entry.path}
              entry={entry}
              runtime={runtime}
              theme={theme}
              onLaunch={launchEntry}
            />
          ))}
        </div>
      </div>
    </div>
  );

  return (
    <div
      id="start-menu"
      className={`${styles["start-menu"]} ${styles["shell-menu"]} shell-menu layout-${layout}`}
      ref={menuRef}
      role="menu"
    >
      {layout === "classic" && renderClassic()}
      {layout === "experience" && renderExperience()}
      {layout === "fluent" && renderFluent()}
    </div>
  );
}

function ShortcutButton({
  entry,
  runtime,
  theme,
  onLaunch,
}: {
  entry: DirectoryEntry;
  runtime: RuntimeBridge | null;
  theme: string;
  onLaunch: (entry: DirectoryEntry) => Promise<void>;
}) {
  const [iconSrc, setIconSrc] = useState(ICON_PLACEHOLDER);

  useEffect(() => {
    let alive = true;
    if (!runtime) {
      setIconSrc(`/themes/${theme}/icons/shell/folder.webp`);
      return;
    }

    resolveIcon(entry, runtime)
      .then((resolved) => {
        if (!alive) return;
        setIconSrc(resolved.src);
      })
      .catch(() => {
        if (!alive) return;
        setIconSrc(`/themes/${theme}/icons/shell/folder.webp`);
      });

    return () => {
      alive = false;
    };
  }, [entry, runtime, theme]);

  return (
    <button
      className={`${styles["shell-menu-item"]} shell-menu-item`}
      type="button"
      role="menuitem"
      onClick={() => {
        void onLaunch(entry);
      }}
    >
      <img
        src={iconSrc}
        alt=""
        className={`${styles["start-item-icon"]} start-item-icon`}
        draggable={false}
        onError={(e) => {
          e.currentTarget.src = `/themes/${theme}/icons/shell/folder.webp`;
        }}
      />
      <span className={`${styles["menu-item-label"]} menu-item-label`}>{displayLabel(entry.name)}</span>
    </button>
  );
}

function ShortcutTile({
  entry,
  runtime,
  theme,
  onLaunch,
}: {
  entry: DirectoryEntry;
  runtime: RuntimeBridge | null;
  theme: string;
  onLaunch: (entry: DirectoryEntry) => Promise<void>;
}) {
  const [iconSrc, setIconSrc] = useState(ICON_PLACEHOLDER);

  useEffect(() => {
    let alive = true;
    if (!runtime) {
      setIconSrc(`/themes/${theme}/icons/shell/folder.webp`);
      return;
    }

    resolveIcon(entry, runtime)
      .then((resolved) => {
        if (!alive) return;
        setIconSrc(resolved.src);
      })
      .catch(() => {
        if (!alive) return;
        setIconSrc(`/themes/${theme}/icons/shell/folder.webp`);
      });

    return () => {
      alive = false;
    };
  }, [entry, runtime, theme]);

  return (
    <button
      className={`${styles["start-tile"]} start-tile`}
      type="button"
      onClick={() => {
        void onLaunch(entry);
      }}
    >
      <img
        src={iconSrc}
        alt=""
        className={`${styles["start-tile-icon"]} start-tile-icon`}
        draggable={false}
        onError={(e) => {
          e.currentTarget.src = `/themes/${theme}/icons/shell/folder.webp`;
        }}
      />
      <span className={`${styles["start-tile-label"]} start-tile-label`}>{displayLabel(entry.name)}</span>
    </button>
  );
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
  return [...entries].sort((a, b) =>
    displayLabel(a.name).localeCompare(displayLabel(b.name), undefined, { sensitivity: "base" }),
  );
}
