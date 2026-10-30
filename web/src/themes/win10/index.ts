import "./win10.css";
import "./shell.css";
import { registerTheme } from "@/shared/lib/theme-registry";

export const win10Theme = registerTheme({
  id: "win10",
  name: "Windows 10",
  description: "Solid, square and minimal, with an accent-coloured frame and the full-height Start menu.",
  startMenu: "modern",
  preferredWallpaper: "win10",
  preferredSoundPack: "win10",
  preferredIconPack: "win10",
  preview: {
    desktop: "linear-gradient(135deg, #00264d, #004c99 55%, #0063b1)",
    frame: "#fff",
    titleText: "#000",
    body: "#fff",
    accent: "#0078d7",
    taskbar: "#101010",
  },
});
