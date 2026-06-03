import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";

export interface MenuItem {
  label: string;
  action?: () => void;
  disabled?: boolean;
  danger?: boolean;
  checked?: boolean;
  icon?: React.ReactNode;
  children?: MenuItem[];
}

export const SEPARATOR: MenuItem = { label: "---" };

interface ContextMenuProps {
  x: number;
  y: number;
  items: MenuItem[];
  onClose: () => void;
}

type MenuPosition = {
  left: number;
  top: number;
};

const MENU_MIN_WIDTH = 214;
const SUBMENU_GAP = -3;
const VIEWPORT_PADDING = 4;

export function ContextMenu({ x, y, items, onClose }: ContextMenuProps) {
  const rootRef = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState<MenuPosition>({ left: x, top: y });

  useLayoutEffect(() => {
    const el = rootRef.current;
    if (!el) return;

    const next = clampMenuToViewport(x, y, el);
    setPosition(next);
  }, [x, y, items]);

  useEffect(() => {
    const onDown = (e: MouseEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) {
        onClose();
      }
    };

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };

    const onScroll = () => {
      onClose();
    };

    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    window.addEventListener("blur", onClose);
    window.addEventListener("scroll", onScroll, true);
    window.addEventListener("resize", onClose);

    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
      window.removeEventListener("blur", onClose);
      window.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("resize", onClose);
    };
  }, [onClose]);

  const menu = (
    <div
      ref={rootRef}
      className="fixed z-[99999] select-none"
      style={{
        left: position.left,
        top: position.top,
      }}
      onContextMenu={(e) => e.preventDefault()}
    >
      <ContextMenuLevel items={items} onClose={onClose} depth={0} />
    </div>
  );

  return createPortal(menu, document.body);
}

function ContextMenuLevel({
  items,
  onClose,
  depth,
}: {
  items: MenuItem[];
  onClose: () => void;
  depth: number;
}) {
  const menuRef = useRef<HTMLUListElement>(null);
  const [openIndex, setOpenIndex] = useState<number | null>(null);
  const [submenuSide, setSubmenuSide] = useState<"right" | "left">("right");

  const openSubmenu = (index: number, target: HTMLElement) => {
    const item = items[index];

    if (!item?.children?.length || item.disabled) {
      setOpenIndex(null);
      return;
    }

    const rect = target.getBoundingClientRect();
    const estimatedWidth = MENU_MIN_WIDTH;

    const hasRightSpace =
      rect.right + SUBMENU_GAP + estimatedWidth <=
      window.innerWidth - VIEWPORT_PADDING;

    setSubmenuSide(hasRightSpace ? "right" : "left");
    setOpenIndex(index);
  };

  const closeSubmenu = () => {
    setOpenIndex(null);
  };

  return (
    <ul
      ref={menuRef}
      className={[
        "relative m-0 list-none rounded-none",
        "min-w-[214px] py-[3px] px-0",
        "bg-[rgba(43,43,43,0.98)]",
        "border border-[rgba(255,255,255,0.18)]",
        "shadow-[0_4px_16px_rgba(0,0,0,0.45)]",
        "text-[#f2f2f2] font-[var(--system-font)] text-[12px]",
      ].join(" ")}
      role="menu"
      onMouseLeave={() => {
        if (depth > 0) return;
      }}
    >
      {items.map((item, index) => {
        if (item.label === "---") {
          return (
            <li
              key={`separator-${index}`}
              className="h-px my-[3px] mx-[28px] bg-[rgba(255,255,255,0.16)]"
              role="separator"
            />
          );
        }

        const hasChildren = Boolean(item.children?.length);
        const isOpen = openIndex === index;

        return (
          <li
            key={`${item.label}-${index}`}
            className="relative mx-[2px]"
            onMouseEnter={(e) => openSubmenu(index, e.currentTarget)}
            onMouseLeave={() => {
              if (!hasChildren) closeSubmenu();
            }}
          >
            <button
              type="button"
              role="menuitem"
              aria-haspopup={hasChildren || undefined}
              aria-expanded={hasChildren ? isOpen : undefined}
              aria-disabled={item.disabled}
              disabled={item.disabled}
              className={[
                "relative w-full min-h-[26px]",
                "grid grid-cols-[24px_1fr_18px] items-center",
                "gap-0 rounded-none border-0 bg-transparent",
                "pl-0 pr-[6px] py-0",
                "text-left text-[12px] leading-none font-normal",
                "text-[#f2f2f2] cursor-default",
                "disabled:text-[rgba(255,255,255,0.38)] disabled:pointer-events-none",
                "hover:bg-[rgba(255,255,255,0.13)]",
                "focus-visible:outline focus-visible:outline-1 focus-visible:outline-white focus-visible:-outline-offset-[2px]",
                item.danger
                  ? "text-[#ffb3bd] hover:bg-[rgba(232,17,35,0.22)]"
                  : "",
                isOpen ? "bg-[rgba(255,255,255,0.13)]" : "",
              ].join(" ")}
              onMouseDown={(e) => {
                e.stopPropagation();

                if (item.disabled) return;

                if (hasChildren) {
                  openSubmenu(index, e.currentTarget);
                  return;
                }

                if (!item.action) return;

                onClose();
                item.action();
              }}
            >
              <span className="w-6 h-[26px] grid place-items-center text-[#f2f2f2]">
                {item.checked ? (
                  <span className="text-[13px] leading-none">✓</span>
                ) : item.icon ? (
                  item.icon
                ) : null}
              </span>

              <span className="min-w-0 overflow-hidden text-ellipsis whitespace-nowrap">
                {item.label}
              </span>

              <span className="w-[18px] h-[26px] grid place-items-center text-[15px] text-[#f2f2f2]">
                {hasChildren ? "›" : null}
              </span>
            </button>

            {hasChildren && isOpen && (
              <div
                className="absolute top-0"
                style={{
                  left:
                    submenuSide === "right"
                      ? `calc(100% + ${SUBMENU_GAP}px)`
                      : "auto",
                  right:
                    submenuSide === "left"
                      ? `calc(100% + ${SUBMENU_GAP}px)`
                      : "auto",
                }}
              >
                <ContextMenuLevel
                  items={item.children ?? []}
                  onClose={onClose}
                  depth={depth + 1}
                />
              </div>
            )}
          </li>
        );
      })}
    </ul>
  );
}

function clampMenuToViewport(x: number, y: number, el: HTMLElement): MenuPosition {
  const rect = el.getBoundingClientRect();

  let left = x;
  let top = y;

  if (left + rect.width > window.innerWidth - VIEWPORT_PADDING) {
    left = window.innerWidth - rect.width - VIEWPORT_PADDING;
  }

  if (top + rect.height > window.innerHeight - VIEWPORT_PADDING) {
    top = window.innerHeight - rect.height - VIEWPORT_PADDING;
  }

  left = Math.max(VIEWPORT_PADDING, left);
  top = Math.max(VIEWPORT_PADDING, top);

  return { left, top };
}