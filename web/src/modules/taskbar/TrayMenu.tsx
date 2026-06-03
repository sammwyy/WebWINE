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
    <div id="tray-menu" className="fixed bottom-[44px] z-[9500] bg-[var(--menu-bg)] border border-[var(--menu-border)] rounded-[var(--menu-radius)] shadow-[0_12px_36px_rgba(0,0,0,0.35)] text-[var(--menu-text)] backdrop-blur-md right-[72px] w-[180px] h-[110px] p-2" ref={menuRef} role="menu" />
  );
}
