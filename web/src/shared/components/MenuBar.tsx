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
      className="h-[28px] flex items-center gap-0 px-1 bg-[#181818] border-b border-[#2b2b2b] select-none"
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
            className={[
              "h-[24px] px-3 rounded-none border border-transparent",
              "bg-transparent text-[#f2f2f2] text-[12px] leading-none",
              "cursor-default hover:bg-[rgba(255,255,255,0.10)]",
              "disabled:text-[rgba(255,255,255,0.38)] disabled:pointer-events-none",
              active ? "bg-[rgba(255,255,255,0.14)] border-[rgba(255,255,255,0.10)]" : "",
            ].join(" ")}
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
