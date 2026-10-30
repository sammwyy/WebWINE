import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { ChevronRightGlyph } from "@/shared/icons";

function CheckGlyph() {
  return (
    <svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true">
      <path d="M1.5 5.2 4 7.6 8.6 2.4" fill="none" stroke="currentColor" strokeWidth="1.5" />
    </svg>
  );
}

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

    // Only close on page-level scroll. Nested scroll containers (console,
    // explorer, etc.) fire capture-phase scroll on `window` and must not
    // dismiss the menu while their content updates.
    const onScroll = (e: Event) => {
      const t = e.target;
      if (
        t === document ||
        t === document.documentElement ||
        t === document.body
      ) {
        onClose();
      }
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
      onContextMenu={(e) => {
        e.preventDefault();
        e.stopPropagation();
      }}
      onMouseDown={(e) => {
        e.stopPropagation();
      }}
      onClick={(e) => {
        e.stopPropagation();
      }}
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
      className="ww-menu"
      role="menu"
      onMouseLeave={() => {
        if (depth > 0) return;
      }}
    >
      {items.map((item, index) => {
        if (item.label === "---") {
          return <li key={`separator-${index}`} className="ww-menu-sep" role="separator" />;
        }

        const hasChildren = Boolean(item.children?.length);
        const isOpen = openIndex === index;

        return (
          <li
            key={`${item.label}-${index}`}
            className="ww-menu-entry"
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
              className={["ww-menu-item", item.danger && "danger", isOpen && "open"].filter(Boolean).join(" ")}
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
              <span className="ww-menu-check">
                {item.checked ? <CheckGlyph /> : item.icon ? item.icon : null}
              </span>

              <span className="ww-menu-label">{item.label}</span>

              <span />

              <span className="ww-menu-arrow">{hasChildren ? <ChevronRightGlyph /> : null}</span>
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
