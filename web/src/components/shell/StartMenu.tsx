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
      <span className={`${styles["start-item-icon"]} start-item-icon fallback-icon start-item-icon-${action}`} aria-hidden="true" />
      {label}
    </button>
  );

  const renderPlainItem = (
    label: string,
    className = "",
    options: { arrow?: boolean; disabled?: boolean; action?: StartMenuAction } = {},
  ) => (
    <button
      key={label}
      className={`${styles["shell-menu-item"]} shell-menu-item ${className}`}
      type="button"
      role="menuitem"
      aria-disabled={options.disabled || undefined}
      onClick={() => {
        if (options.disabled) return;
        if (options.action) {
          onClose();
          onAction(options.action);
        }
      }}
    >
      <span className={`${styles["start-item-icon"]} start-item-icon fallback-icon`} aria-hidden="true" />
      <span className={`${styles["menu-item-label"]} menu-item-label`}>{label}</span>
      {options.arrow && <span className={`${styles["menu-item-arrow"]} menu-item-arrow`} aria-hidden="true">&gt;</span>}
    </button>
  );

  const runAction = (action: StartMenuAction) => {
    onClose();
    onAction(action);
  };

  const renderRailButton = (
    kind: "menu" | "user" | "settings" | "power",
    label: string,
    action?: StartMenuAction,
  ) => (
    <button
      className={`${styles["rail-btn"]} rail-btn rail-btn-${kind}`}
      type="button"
      title={label}
      aria-label={label}
      onClick={() => (action ? runAction(action) : onClose())}
    >
      <span className={`${styles["rail-icon"]} rail-icon`} aria-hidden="true" />
    </button>
  );

  const renderTile = (
    action: StartMenuAction,
    label: string,
    icon: "app" | "file" | "folder",
    size: "square" | "wide" = "square",
  ) => (
    <button
      key={action}
      className={`${styles["start-tile"]} start-tile start-tile-${size}`}
      type="button"
      onClick={() => runAction(action)}
    >
      {icon === "app" ? (
        <img
          src={`/themes/${theme}/icons/apps/${action}.webp`}
          alt=""
          className={`${styles["start-tile-icon"]} start-tile-icon`}
          draggable={false}
          onError={(e) => { e.currentTarget.style.display = "none"; }}
        />
      ) : (
        <span
          className={`${styles["start-tile-icon"]} start-tile-icon start-tile-icon-${icon}`}
          aria-hidden="true"
        />
      )}
      <span className={`${styles["start-tile-label"]} start-tile-label`}>{label}</span>
    </button>
  );

  const renderClassic = () => (
    <div className="start-menu-layout classic">
      <div className={`${styles["start-menu-sidebar"]} start-menu-sidebar`}>
        <span className={`${styles["start-menu-sidebar-text"]} start-menu-sidebar-text`}>WebWINE</span>
      </div>
      <div className={`${styles["start-menu-content"]} start-menu-content classic-menu-content`}>
        {renderPlainItem("Programs", "classic-programs", { arrow: true, action: "explorer" })}
        {renderPlainItem("Documents", "classic-documents", { arrow: true, action: "explorer" })}
        {renderPlainItem("Settings", "classic-settings", { arrow: true, action: "themes" })}
        {renderPlainItem("Find", "classic-find", { arrow: true })}
        {renderPlainItem("Help", "classic-help", { disabled: true })}
        {renderPlainItem("Run...", "classic-run", { action: "upload-file" })}
        <div className={`${styles["shell-menu-separator"]} shell-menu-separator`} />
        {renderPlainItem("Log Off WebWINE User...", "classic-logoff")}
        {renderPlainItem("Shut Down...", "classic-shutdown")}
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
          <div className={`${styles["xp-pinned"]} xp-pinned`}>
            <div className={`${styles["shell-menu-title"]} shell-menu-title`}>Pinned programs</div>
            {renderAppItem("explorer", "File Explorer")}
            {renderAppItem("themes", "Set Program Access and Defaults")}
          </div>
          <div className={`${styles["shell-menu-separator"]} shell-menu-separator`} />
          <div className={`${styles["xp-recent"]} xp-recent`}>
            <div className={`${styles["shell-menu-title"]} shell-menu-title`}>Most frequently used programs</div>
            {renderOpItem("upload-file", "Upload a File")}
            {renderOpItem("upload-folder", "Upload a Folder")}
          </div>
          <div className={`${styles["xp-all-programs"]} xp-all-programs`}>
            <button className={`${styles["xp-all-programs-btn"]} xp-all-programs-btn`} type="button" onClick={() => runAction("explorer")}>
              All Programs
              <span className={`${styles["xp-all-programs-arrow"]} xp-all-programs-arrow`} aria-hidden="true">&gt;</span>
            </button>
          </div>
        </div>
        <div className={`${styles["start-menu-right"]} start-menu-right`}>
          {renderPlainItem("My Documents", "xp-documents", { action: "explorer" })}
          {renderPlainItem("My Recent Documents", "xp-recent-documents", { arrow: true })}
          {renderPlainItem("My Pictures", "xp-pictures", { action: "explorer" })}
          {renderPlainItem("My Music", "xp-music", { action: "explorer" })}
          {renderPlainItem("My Computer", "xp-computer", { action: "explorer" })}
          <div className={`${styles["shell-menu-separator"]} shell-menu-separator`} />
          {renderPlainItem("Control Panel", "xp-control-panel", { action: "themes" })}
          {renderPlainItem("Printers and Faxes", "xp-printers", { disabled: true })}
          <div className={`${styles["shell-menu-separator"]} shell-menu-separator`} />
          {renderPlainItem("Help and Support", "xp-help", { disabled: true })}
          {renderPlainItem("Search", "xp-search")}
          {renderPlainItem("Run...", "xp-run", { action: "upload-file" })}
        </div>
      </div>
      <div className={`${styles["xp-footer"]} xp-footer`}>
        <button className={`${styles["xp-footer-btn"]} xp-footer-btn xp-logoff`} type="button" onClick={onClose}>
          Log Off
        </button>
        <button className={`${styles["xp-footer-btn"]} xp-footer-btn xp-shutdown`} type="button" onClick={onClose}>
          Turn Off Computer
        </button>
      </div>
    </div>
  );

  const renderFluent = () => (
    <div className="start-menu-layout fluent">
      <div className={`${styles["start-menu-rail"]} start-menu-rail`}>
        <div className={`${styles["rail-top"]} rail-top`}>
          {renderRailButton("menu", "Expand")}
        </div>
        <div className={`${styles["rail-bottom"]} rail-bottom`}>
          {renderRailButton("user", "WebWINE User")}
          {renderRailButton("settings", "Themes", "themes")}
          {renderRailButton("power", "Close")}
        </div>
      </div>
      <div className={`${styles["start-menu-apps"]} start-menu-apps`}>
        <div className={`${styles["start-menu-section-title"]} start-menu-section-title`}>All apps</div>
        <div className={`${styles["start-menu-alpha"]} start-menu-alpha`}>W</div>
        {renderAppItem("explorer", "File Explorer")}
        {renderAppItem("themes", "Themes")}
        <div className={`${styles["start-menu-alpha"]} start-menu-alpha`}>F</div>
        {renderOpItem("upload-file", "Upload File")}
        {renderOpItem("upload-folder", "Upload Folder")}
      </div>
      <div className={`${styles["start-menu-tiles"]} start-menu-tiles`}>
        <div className={`${styles["tile-group"]} tile-group`}>
          <div className={`${styles["tile-group-title"]} tile-group-title`}>Life at a glance</div>
          <div className={`${styles["tile-grid"]} tile-grid`}>
            {renderTile("explorer", "File Explorer", "app", "wide")}
            {renderTile("themes", "Themes", "app")}
          </div>
        </div>
        <div className={`${styles["tile-group"]} tile-group`}>
          <div className={`${styles["tile-group-title"]} tile-group-title`}>WebWINE</div>
          <div className={`${styles["tile-grid"]} tile-grid`}>
            {renderTile("upload-file", "Upload File", "file")}
            {renderTile("upload-folder", "Upload Folder", "folder")}
          </div>
        </div>
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
