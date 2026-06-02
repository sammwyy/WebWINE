export interface MenuItem {
  label: string;
  action?: () => void;
  disabled?: boolean;
  danger?: boolean;
}

export const SEPARATOR: MenuItem = { label: "---" };

let activeMenu: HTMLElement | null = null;

export function showContextMenu(x: number, y: number, items: MenuItem[]) {
  dismissMenu();

  const menu = document.createElement("ul");
  menu.className = "ctx-menu";
  menu.style.left = `${x}px`;
  menu.style.top = `${y}px`;

  for (const item of items) {
    if (item.label === "---") {
      const sep = document.createElement("li");
      sep.className = "ctx-sep";
      menu.appendChild(sep);
      continue;
    }

    const li = document.createElement("li");
    li.className = "ctx-item";
    if (item.disabled) li.classList.add("ctx-disabled");
    if (item.danger)   li.classList.add("ctx-danger");
    li.textContent = item.label;

    if (!item.disabled && item.action) {
      li.addEventListener("mousedown", (e) => {
        e.stopPropagation();
        dismissMenu();
        item.action!();
      });
    }

    menu.appendChild(li);
  }

  document.body.appendChild(menu);
  activeMenu = menu;

  // reposition if it clips off screen
  requestAnimationFrame(() => {
    const rect = menu.getBoundingClientRect();
    if (rect.right > window.innerWidth)  menu.style.left = `${x - rect.width}px`;
    if (rect.bottom > window.innerHeight) menu.style.top  = `${y - rect.height}px`;
  });

  const onDown = (e: MouseEvent) => {
    if (!menu.contains(e.target as Node)) dismissMenu();
  };
  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape") dismissMenu();
  };

  document.addEventListener("mousedown", onDown, { once: false });
  document.addEventListener("keydown", onKey, { once: true });

  (menu as unknown as { _cleanup: () => void })._cleanup = () => {
    document.removeEventListener("mousedown", onDown);
    document.removeEventListener("keydown", onKey);
  };
}

export function dismissMenu() {
  if (!activeMenu) return;
  const m = activeMenu as unknown as { _cleanup?: () => void; remove: () => void };
  m._cleanup?.();
  m.remove();
  activeMenu = null;
}
