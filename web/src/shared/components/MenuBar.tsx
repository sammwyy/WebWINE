import { useState } from "react";
import { ContextMenu, type MenuItem } from "@/modules/desktop/ContextMenu";

export interface MenuBarItem {
  label: string;
  items: MenuItem[];
  disabled?: boolean;
}

export function MenuBar({ items }: { items: MenuBarItem[] }) {
  const [open, setOpen] = useState<{
    label: string;
    x: number;
    y: number;
    items: MenuItem[];
  } | null>(null);

  const openMenu = (item: MenuBarItem, target: HTMLElement) => {
    if (item.disabled) return;
    const rect = target.getBoundingClientRect();
    setOpen({
      label: item.label,
      x: rect.left,
      y: rect.bottom,
      items: item.items,
    });
  };

  return (
    <div
      className="ww-menubar"
      role="menubar"
      onContextMenu={(e) => {
        e.preventDefault();
        e.stopPropagation();
      }}
    >
      {items.map((item) => {
        const active = open?.label === item.label;
        return (
          <button
            key={item.label}
            type="button"
            role="menuitem"
            aria-haspopup="menu"
            aria-expanded={active}
            disabled={item.disabled}
            className={`ww-menubar-item ${active ? "active" : ""}`}
            onMouseDown={(e) => {
              e.preventDefault();
              e.stopPropagation();
              openMenu(item, e.currentTarget);
            }}
            onMouseEnter={(e) => {
              if (open) openMenu(item, e.currentTarget);
            }}
          >
            {item.label}
          </button>
        );
      })}

      {open && (
        <ContextMenu
          x={open.x}
          y={open.y}
          items={open.items}
          onClose={() => setOpen(null)}
        />
      )}
    </div>
  );
}
