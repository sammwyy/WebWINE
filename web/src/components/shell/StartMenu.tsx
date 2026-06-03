/**
 * StartMenu — floating panel anchored to the start button.
 *
 * Receives an onClose callback and dispatches named actions upward.
 */

import { useEffect, useRef } from "react";
import { useThemeStore } from "../../stores/useThemeStore.js";

export type StartMenuAction =
  | "explorer"
  | "themes"
  | "upload-file"
  | "upload-folder";

interface StartMenuProps {
  onAction: (action: StartMenuAction) => void;
  onClose: () => void;
}

export function StartMenu({ onAction, onClose }: StartMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);
  const { theme } = useThemeStore();

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (
        !menuRef.current?.contains(target) &&
        !target.closest("#start-button")
      ) {
        onClose();
      }
    };
    document.addEventListener("click", handler);
    return () => document.removeEventListener("click", handler);
  }, [onClose]);

  return (
    <div id="start-menu" className="shell-menu" ref={menuRef} role="menu">
      <div className="start-menu-header">
        <span className="start-menu-mark" aria-hidden="true">
          ww
        </span>
        <span className="start-menu-heading">
          <span className="start-menu-title">WebWINE</span>
          <span className="start-menu-subtitle">Browser shell</span>
        </span>
      </div>

      <div className="shell-menu-title">Apps</div>
      {([
        ["explorer", "File Explorer"],
        ["themes", "Themes"],
      ] as [StartMenuAction, string][]).map(([action, label]) => (
        <button
          key={action}
          className="shell-menu-item"
          type="button"
          role="menuitem"
          onClick={() => {
            onClose();
            onAction(action);
          }}
        >
          <img 
            src={`/themes/${theme}/icons/apps/${action}.webp`} 
            alt="" 
            style={{ width: 24, height: 24, marginRight: 8, objectFit: "contain" }}
            onError={(e) => { e.currentTarget.style.display = "none"; }} 
          />
          {label}
        </button>
      ))}

      <div className="shell-menu-title" style={{ marginTop: 8 }}>Operations</div>
      {([
        ["upload-file", "Upload File"],
        ["upload-folder", "Upload Folder"],
      ] as [StartMenuAction, string][]).map(([action, label]) => (
        <button
          key={action}
          className="shell-menu-item"
          type="button"
          role="menuitem"
          onClick={() => {
            onClose();
            onAction(action);
          }}
        >
          {label}
        </button>
      ))}
    </div>
  );
}
