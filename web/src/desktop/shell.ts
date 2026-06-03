import type { RuntimeBridge } from "../runtime-bridge.js";
import { activateWindowFromTaskbar, getOpenWindows } from "../windows/manager.js";
import { openThemesApp } from "./theme.js";

const DESKTOP_PATH = "C:\\Users\\guest\\Desktop";

export function initShell(runtime: RuntimeBridge) {
  initStartMenu(runtime);
  initTrayMenu();
  initClock();
  renderTaskbarWindows();
  window.addEventListener("webwine:windows-changed", renderTaskbarWindows);
}

function initStartMenu(runtime: RuntimeBridge) {
  const startButton = document.getElementById("start-button") as HTMLButtonElement;
  const startMenu = document.getElementById("start-menu")!;

  startButton.addEventListener("click", (e) => {
    e.stopPropagation();
    closeTrayMenu();
    toggleMenu(startMenu, startButton);
  });

  startMenu.addEventListener("click", (e) => {
    const item = (e.target as HTMLElement).closest<HTMLButtonElement>("[data-action]");
    if (!item) return;

    closeMenu(startMenu, startButton);
    const action = item.dataset.action;
    if (action === "explorer") {
      import("../windows/explorer.js").then((m) => m.openExplorer(DESKTOP_PATH, runtime));
    } else if (action === "themes") {
      openThemesApp();
    } else if (action === "upload-file") {
      window.dispatchEvent(new Event("webwine:upload-file"));
    } else if (action === "upload-folder") {
      window.dispatchEvent(new Event("webwine:upload-folder"));
    }
  });

  document.addEventListener("click", (e) => {
    const target = e.target as HTMLElement;
    if (!target.closest("#start-menu") && !target.closest("#start-button")) {
      closeMenu(startMenu, startButton);
    }
  });
}

function initTrayMenu() {
  const trayButton = document.getElementById("tray-toggle") as HTMLButtonElement;
  const trayMenu = document.getElementById("tray-menu")!;

  trayButton.addEventListener("click", (e) => {
    e.stopPropagation();
    closeStartMenu();
    toggleMenu(trayMenu, trayButton);
  });

  document.addEventListener("click", (e) => {
    const target = e.target as HTMLElement;
    if (!target.closest("#tray-menu") && !target.closest("#tray-toggle")) {
      closeMenu(trayMenu, trayButton);
    }
  });
}

function initClock() {
  function updateClock() {
    const el = document.getElementById("taskbar-clock");
    if (!el) return;
    el.textContent = new Date().toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  updateClock();
  setInterval(updateClock, 1000);
}

function renderTaskbarWindows() {
  const list = document.getElementById("taskbar-window-list")!;
  list.innerHTML = "";

  for (const win of getOpenWindows()) {
    const btn = document.createElement("button");
    btn.className = "taskbar-window-button";
    if (win.active) btn.classList.add("active");
    if (win.minimized) btn.classList.add("minimized");
    if (win.maximized) btn.classList.add("maximized");
    btn.type = "button";
    btn.title = win.title;
    btn.dataset.windowId = win.id;

    if (win.icon) {
      const icon = document.createElement("span");
      icon.className = "taskbar-window-icon";
      icon.textContent = win.icon;
      btn.append(icon);
    }

    const title = document.createElement("span");
    title.className = "taskbar-window-title";
    title.textContent = win.title;
    btn.append(title);

    btn.addEventListener("click", () => activateWindowFromTaskbar(win.id));
    list.append(btn);
  }
}

function toggleMenu(menu: HTMLElement, button: HTMLButtonElement) {
  const open = menu.hidden;
  menu.hidden = !open;
  button.setAttribute("aria-expanded", String(open));
}

function closeMenu(menu: HTMLElement, button: HTMLButtonElement) {
  menu.hidden = true;
  button.setAttribute("aria-expanded", "false");
}

function closeStartMenu() {
  closeMenu(
    document.getElementById("start-menu")!,
    document.getElementById("start-button") as HTMLButtonElement,
  );
}

function closeTrayMenu() {
  closeMenu(
    document.getElementById("tray-menu")!,
    document.getElementById("tray-toggle") as HTMLButtonElement,
  );
}
