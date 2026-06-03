export type ThemeId = "fluent" | "luna" | "classic";

export type TaskbarIconMode = "full" | "only-label" | "only-icon";
export type StartMenuLayout = "classic" | "experience" | "fluent";

export interface ThemeOption {
  id: ThemeId;
  name: string;
  description: string;
  taskbarIcon: TaskbarIconMode;
  startMenuLayout: StartMenuLayout;
}

export const THEMES: ThemeOption[] = [
  {
    id: "fluent",
    name: "Fluent (Win10)",
    description: "Dark square shell with blue focus accents.",
    taskbarIcon: "only-icon",
    startMenuLayout: "fluent",
  },
  {
    id: "luna",
    name: "Luna (WinXP)",
    description: "Bright blue taskbar, rounded chrome, and warm menu panels.",
    taskbarIcon: "full",
    startMenuLayout: "experience",
  },
  {
    id: "classic",
    name: "Classic (Win98)",
    description: "Flat gray controls, beveled borders, and compact chrome.",
    taskbarIcon: "full",
    startMenuLayout: "classic",
  },
];
