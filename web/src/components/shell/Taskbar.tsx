/**
 * Taskbar — the bottom bar with start button, window list, tray, and clock.
 */

import { useState, useCallback, useEffect } from "react";
import { useWindowStore } from "../../stores/useWindowStore.js";
import { useRuntimeStore } from "../../stores/useRuntimeStore.js";
import { useThemeStore } from "../../stores/useThemeStore.js";
import { StartMenu, type StartMenuAction } from "./StartMenu.js";
import { TrayMenu } from "./TrayMenu.js";
import { Clock } from "./Clock.js";
import styles from "./Taskbar.module.css";

const DESKTOP_PATH = "C:\\Users\\guest\\Desktop";

interface TaskbarProps {
  /** Hidden file inputs managed by the parent (Desktop) for uploads. */
  fileInputRef: React.RefObject<HTMLInputElement | null>;
  folderInputRef: React.RefObject<HTMLInputElement | null>;
}

export function Taskbar({ fileInputRef, folderInputRef }: TaskbarProps) {
  const [startOpen, setStartOpen] = useState(false);
  const [trayOpen, setTrayOpen] = useState(false);
  const [orbStatus, setOrbStatus] = useState<"loading" | "loaded" | "error">("loading");
  const [orbHover, setOrbHover] = useState(false);

  const { windows, activeId, activateFromTaskbar } = useWindowStore();
  const { runtime } = useRuntimeStore();
  const { theme, getEffectiveTaskbarIconMode } = useThemeStore();
  const mode = getEffectiveTaskbarIconMode();

  useEffect(() => {
    setOrbStatus("loading");
    const img = new Image();
    img.onload = () => setOrbStatus("loaded");
    img.onerror = () => setOrbStatus("error");
    img.src = `/themes/${theme}/orb.webp`;
  }, [theme]);

  const handleStartAction = useCallback(
    (action: StartMenuAction) => {
      if (!runtime) return;
      if (action === "explorer") {
        import("../../apps/explorer/ExplorerApp.js").then((m) =>
          m.openExplorer(DESKTOP_PATH, runtime),
        );
      } else if (action === "themes") {
        import("../../apps/theme-switcher/ThemeSwitcherApp.js").then((m) =>
          m.openThemeSwitcher(),
        );
      } else if (action === "upload-file") {
        fileInputRef.current?.click();
      } else if (action === "upload-folder") {
        folderInputRef.current?.click();
      }
    },
    [runtime, fileInputRef, folderInputRef],
  );

  return (
    <div id="taskbar" className={styles.taskbar}>
      <button
        id="start-button"
        type="button"
        className={`${styles["start-button"]} ${orbStatus === "loaded" ? styles["has-orb"] + " has-orb" : ""}`}
        aria-haspopup="true"
        aria-expanded={startOpen}
        onMouseEnter={() => setOrbHover(true)}
        onMouseLeave={() => setOrbHover(false)}
        onClick={(e) => {
          e.stopPropagation();
          setTrayOpen(false);
          setStartOpen((v) => !v);
        }}
      >
        {orbStatus === "loaded" ? (
          <img 
            src={`/themes/${theme}/orb.webp`}
            alt="Start"
            draggable={false}
            className={`${styles["start-orb-sprite-img"]} start-orb-sprite-img`}
            style={{
              transform: `translateY(${startOpen ? '-66.666%' : (orbHover ? '-33.333%' : '0')})`
            }}
          />
        ) : orbStatus === "error" ? (
          <>
            <span className={`${styles["start-mark"]} start-mark`} aria-hidden="true" />
            <span>WebWINE</span>
          </>
        ) : null}
      </button>

      {startOpen && (
        <StartMenu
          onAction={handleStartAction}
          onClose={() => setStartOpen(false)}
        />
      )}

      <div
        id="taskbar-window-list"
        className={styles["taskbar-window-list"]}
        aria-label="Open windows"
        role="toolbar"
      >
        {windows.map((win) => (
          <button
            key={win.id}
            className={[
              styles["taskbar-window-button"],
              "taskbar-window-button",
              win.id === activeId ? styles.active + " active" : "",
              win.minimized ? styles.minimized + " minimized" : "",
              win.maximized ? styles.maximized + " maximized" : "",
              styles[`icon-mode-${mode}`],
              `icon-mode-${mode}`
            ]
              .filter(Boolean)
              .join(" ")}
            type="button"
            title={win.title}
            onClick={() => activateFromTaskbar(win.id)}
          >
            {(mode === "full" || mode === "only-icon") && win.icon && (
              <span className={`${styles["taskbar-window-icon"]} taskbar-window-icon`} aria-hidden="true">
                {win.icon.includes("/") ? (
                  <img src={win.icon} alt="" style={{width: 16, height: 16, objectFit: "contain"}} draggable={false} onError={(e) => { e.currentTarget.style.display = "none"; }} />
                ) : (
                  win.icon
                )}
              </span>
            )}
            {(mode === "full" || mode === "only-label") && (
              <span className={`${styles["taskbar-window-title"]} taskbar-window-title`}>{win.title}</span>
            )}
          </button>
        ))}
      </div>

      <div id="taskbar-tray" className={styles["taskbar-tray"]}>
        <button
          id="tray-toggle"
          className={styles["tray-toggle"]}
          type="button"
          aria-label="Show hidden icons"
          aria-haspopup="true"
          aria-expanded={trayOpen}
          onClick={(e) => {
            e.stopPropagation();
            setStartOpen(false);
            setTrayOpen((v) => !v);
          }}
        >
          ^
        </button>

        {trayOpen && <TrayMenu onClose={() => setTrayOpen(false)} />}

        <Clock />
      </div>
    </div>
  );
}
