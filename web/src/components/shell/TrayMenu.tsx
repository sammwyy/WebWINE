/**
 * TrayMenu — the floating panel that appears when the tray toggle is clicked.
 * Currently empty; reserved for future tray icons.
 */

import { useEffect, useRef } from "react";

interface TrayMenuProps {
  onClose: () => void;
}

export function TrayMenu({ onClose }: TrayMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (
        !menuRef.current?.contains(target) &&
        !target.closest("#tray-toggle")
      ) {
        onClose();
      }
    };
    document.addEventListener("click", handler);
    return () => document.removeEventListener("click", handler);
  }, [onClose]);

  return (
    <div id="tray-menu" className="shell-menu" ref={menuRef} role="menu" />
  );
}
