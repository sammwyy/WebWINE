import "../win10/win10.css";
import "../win10/shell.css";
import "./win10dark.css";
import { registerTheme } from "@/shared/lib/theme-registry";

export const win10DarkTheme = registerTheme({
  id: "win10-dark",
  name: "Windows 10 (Dark)",
  description: "The original WebWINE look: Windows 10 in dark mode with the WebWINE wallpaper and the full-height Start menu.",
  inherits: ["win10"],
  colorScheme: "dark",
  startMenu: "modern",
  preferredWallpaper: "win10",
  preferredSoundPack: "win10",
  preferredIconPack: "win10",
  preview: {
    desktop: "#0f1014",
    frame: "#202020",
    titleText: "#fff",
    body: "#111",
    accent: "#0078d7",
    taskbar: "#101010",
  },
});
