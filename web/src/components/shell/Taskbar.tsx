/**
 * Taskbar — the bottom bar with start button, window list, tray, and clock.
 */

import { useState, useEffect } from "react";
import { useWindowStore } from "../../stores/useWindowStore.js";
import { useThemeStore } from "../../stores/useThemeStore.js";
import { StartMenu } from "./StartMenu.js";
import { TrayMenu } from "./TrayMenu.js";
import { Clock } from "./Clock.js";
import styles from "./Taskbar.module.css";
import { SHELL_ACTION_EVENT } from "../../lib/guest-launch.js";
import type { ShellActionDetail } from "../../lib/shortcut-target.js";

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
  const { theme, getEffectiveTaskbarIconMode } = useThemeStore();
  const mode = getEffectiveTaskbarIconMode();

  useEffect(() => {
    setOrbStatus("loading");
    const img = new Image();
    img.onload = () => setOrbStatus("loaded");
    img.onerror = () => setOrbStatus("error");
    img.src = `/themes/${theme}/orb.webp`;
  }, [theme]);

  useEffect(() => {
    const onShellAction = (e: Event) => {
      const detail = (e as CustomEvent<ShellActionDetail>).detail;
      if (!detail) return;
      if (detail.action === "upload-file") {
        fileInputRef.current?.click();
      } else if (detail.action === "upload-folder") {
        folderInputRef.current?.click();
      }
    };

    window.addEventListener(SHELL_ACTION_EVENT, onShellAction);
    return () => window.removeEventListener(SHELL_ACTION_EVENT, onShellAction);
  }, [fileInputRef, folderInputRef]);

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
