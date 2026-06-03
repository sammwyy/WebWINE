import "./ContextMenu.css";
/**
 * ContextMenu — a floating portal-rendered context menu.
 *
 * Renders into document.body via a React portal so it always sits above
 * all other content regardless of stacking contexts.
 */

import { useEffect, useRef } from "react";
import { createPortal } from "react-dom";

export interface MenuItem {
  label: string;
  action?: () => void;
  disabled?: boolean;
  danger?: boolean;
}

export const SEPARATOR: MenuItem = { label: "---" };

interface ContextMenuProps {
  x: number;
  y: number;
  items: MenuItem[];
  onClose: () => void;
}

export function ContextMenu({ x, y, items, onClose }: ContextMenuProps) {
  const menuRef = useRef<HTMLUListElement>(null);

  // Reposition if the menu clips off-screen after initial paint.
  useEffect(() => {
    const el = menuRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    if (rect.right > window.innerWidth) {
      el.style.left = `${x - rect.width}px`;
    }
    if (rect.bottom > window.innerHeight) {
      el.style.top = `${y - rect.height}px`;
    }
  }, [x, y]);

  // Close on outside click or Escape.
  useEffect(() => {
    const onDown = (e: MouseEvent) => {
      if (!menuRef.current?.contains(e.target as Node)) onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [onClose]);

  const menu = (
    <ul
      ref={menuRef}
      className="ctx-menu"
      style={{ left: x, top: y }}
      role="menu"
    >
      {items.map((item, i) => {
        if (item.label === "---") {
          return <li key={i} className="ctx-sep" role="separator" />;
        }
        return (
          <li
            key={i}
            className={[
              "ctx-item",
              item.disabled ? "ctx-disabled" : "",
              item.danger ? "ctx-danger" : "",
            ]
              .filter(Boolean)
              .join(" ")}
            role="menuitem"
            aria-disabled={item.disabled}
            onMouseDown={(e) => {
              if (item.disabled || !item.action) return;
              e.stopPropagation();
              onClose();
              item.action();
            }}
          >
            {item.label}
          </li>
        );
      })}
    </ul>
  );

  return createPortal(menu, document.body);
}
