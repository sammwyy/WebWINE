/**
 * TrayMenu — the flyout that appears when the tray toggle is clicked:
 * a quick theme switcher plus shortcuts to the Control Panel.
 */

import { useEffect, useRef } from "react";
import { ThemeSwitcher } from "@/shared/ui";
import { useRuntimeStore } from "@/state/runtimeStore";
import { openControlPanel } from "@/apps/control-panel/ControlPanelApp";

interface TrayMenuProps {
  onClose: () => void;
}

export function TrayMenu({ onClose }: TrayMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);
  const runtime = useRuntimeStore((s) => s.runtime);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (!menuRef.current?.contains(target) && !target.closest("#tray-toggle")) {
        onClose();
      }
    };
    document.addEventListener("click", handler);
    return () => document.removeEventListener("click", handler);
  }, [onClose]);

  return (
    <div id="tray-menu" className="ww-tray-menu" ref={menuRef} role="menu">
      <div className="ww-tray-menu-title">Theme</div>
      <ThemeSwitcher />
      {runtime && (
        <span
          className="ww-link self-end"
          onClick={() => {
            openControlPanel(runtime, "personalization");
            onClose();
          }}
        >
          More personalization settings…
        </span>
      )}
    </div>
  );
}
