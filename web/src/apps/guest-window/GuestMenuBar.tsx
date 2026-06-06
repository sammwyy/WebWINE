/**
 * GuestMenuBar — a guest window's menu bar (from SetMenu). It just maps the Win32
 * menu tree to the shared `MenuBar` / `ContextMenu` components so guest-app menus
 * look identical to Explorer and the rest of the shell. Clicking a leaf fires
 * `onCommand(id)`, which the host turns into a WM_COMMAND posted to the WndProc.
 */

import type { MenuItemData } from "@/core/wasm/worker";
import { MenuBar, type MenuBarItem } from "@/shared/components/MenuBar";
import { SEPARATOR, type MenuItem } from "@/modules/desktop/ContextMenu";

function toMenuItems(items: MenuItemData[], onCommand: (id: number) => void): MenuItem[] {
  return items.map((it) => {
    if (it.separator) return SEPARATOR;
    const label = it.text.replace(/&/g, ""); // strip Win32 accelerator markers
    if (it.children.length > 0) {
      return { label, disabled: it.disabled, children: toMenuItems(it.children, onCommand) };
    }
    return { label, disabled: it.disabled, action: () => onCommand(it.id) };
  });
}

export function GuestMenuBar({
  items,
  onCommand,
}: {
  items: MenuItemData[];
  onCommand: (id: number) => void;
}) {
  if (items.length === 0) return null;
  const barItems: MenuBarItem[] = items.map((top) => ({
    label: top.text.replace(/&/g, ""),
    disabled: top.disabled,
    items: toMenuItems(top.children, onCommand),
  }));
  return <MenuBar items={barItems} />;
}
