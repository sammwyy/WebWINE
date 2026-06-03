import { useEffect, useRef } from "react";
import { useThemeStore } from "../../stores/useThemeStore.js";
import styles from "./StartMenu.module.css";

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
  const { theme, getEffectiveStartMenuLayout } = useThemeStore();
  const layout = getEffectiveStartMenuLayout();

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

  const renderAppItem = (action: StartMenuAction, label: string) => (
    <button
      key={action}
      className={`${styles["shell-menu-item"]} shell-menu-item`}
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
        className={`${styles["start-item-icon"]} start-item-icon`}
        onError={(e) => { e.currentTarget.classList.add("fallback-icon"); e.currentTarget.src = ""; }} 
      />
      {label}
    </button>
  );

  const renderOpItem = (action: StartMenuAction, label: string) => (
    <button
      key={action}
      className={`${styles["shell-menu-item"]} shell-menu-item`}
      type="button"
      role="menuitem"
      onClick={() => {
        onClose();
        onAction(action);
      }}
    >
      <span className={`${styles["start-item-icon"]} start-item-icon fallback-icon`} aria-hidden="true" />
      {label}
    </button>
  );

  const renderClassic = () => (
    <div className="start-menu-layout classic">
      <div className={`${styles["start-menu-sidebar"]} start-menu-sidebar`}>
        <span className={`${styles["start-menu-sidebar-text"]} start-menu-sidebar-text`}>WebWINE</span>
      </div>
      <div className={`${styles["start-menu-content"]} start-menu-content`}>
        <div className={`${styles["shell-menu-title"]} shell-menu-title`}>Apps</div>
        {renderAppItem("explorer", "File Explorer")}
        {renderAppItem("themes", "Themes")}
        <div className={`${styles["shell-menu-separator"]} shell-menu-separator`} />
        <div className={`${styles["shell-menu-title"]} shell-menu-title`}>Operations</div>
        {renderOpItem("upload-file", "Upload File")}
        {renderOpItem("upload-folder", "Upload Folder")}
      </div>
    </div>
  );

  const renderExperience = () => (
    <div className="start-menu-layout experience">
      <div className={`${styles["start-menu-header"]} start-menu-header`}>
        <img src={`/themes/${theme}/icons/shell/msg_inform.webp`} className={`${styles["user-avatar"]} user-avatar`} alt="User" onError={(e) => { e.currentTarget.style.display = "none"; }} />
        <span className={`${styles["user-name"]} user-name`}>WebWINE User</span>
      </div>
      <div className={`${styles["start-menu-columns"]} start-menu-columns`}>
        <div className={`${styles["start-menu-left"]} start-menu-left`}>
          <div className={`${styles["shell-menu-title"]} shell-menu-title`}>Apps</div>
          {renderAppItem("explorer", "File Explorer")}
          {renderAppItem("themes", "Themes")}
        </div>
        <div className={`${styles["start-menu-right"]} start-menu-right`}>
          <div className={`${styles["shell-menu-title"]} shell-menu-title`}>Operations</div>
          {renderOpItem("upload-file", "Upload File")}
          {renderOpItem("upload-folder", "Upload Folder")}
        </div>
      </div>
    </div>
  );

  const renderFluent = () => (
    <div className="start-menu-layout fluent">
      <div className={`${styles["start-menu-rail"]} start-menu-rail`}>
        <button className={`${styles["rail-btn"]} rail-btn`} type="button" title="User">U</button>
        <button className={`${styles["rail-btn"]} rail-btn`} type="button" title="Power">P</button>
      </div>
      <div className={`${styles["start-menu-content"]} start-menu-content`}>
        <div className={`${styles["shell-menu-title"]} shell-menu-title`}>Apps</div>
        {renderAppItem("explorer", "File Explorer")}
        {renderAppItem("themes", "Themes")}
        <div className={`${styles["shell-menu-separator"]} shell-menu-separator`} />
        <div className={`${styles["shell-menu-title"]} shell-menu-title`}>Operations</div>
        {renderOpItem("upload-file", "Upload File")}
        {renderOpItem("upload-folder", "Upload Folder")}
      </div>
    </div>
  );

  return (
    <div id="start-menu" className={`${styles["start-menu"]} ${styles["shell-menu"]} shell-menu layout-${layout}`} ref={menuRef} role="menu">
      {layout === "classic" && renderClassic()}
      {layout === "experience" && renderExperience()}
      {layout === "fluent" && renderFluent()}
    </div>
  );
}
