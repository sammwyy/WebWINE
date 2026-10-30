import "../aero/aero.css";
import "../aero/shell.css";
import "./win7.css";
import { registerTheme } from "@/shared/lib/theme-registry";
import { AeroCaptionGlyph } from "../aero/glyphs";
import { Win7StartOrb } from "../aero/orb";

export const win7Theme = registerTheme({
  id: "win7",
  name: "Windows 7 (Aero)",
  description: 'Lighter "Sky" glass, the icon-only superbar with Show desktop, the blue orb and a light Explorer command bar.',
  inherits: ["aero"],
  taskbarIconSize: 24,
  startMenu: "classic",
  preferredWallpaper: "win7",
  preferredSoundPack: "win7",
  preview: {
    desktop: "radial-gradient(ellipse at 50% 40%, #2b8ad8 0%, #135fb4 35%, #0a3d86 70%, #052456 100%)",
    frame: "linear-gradient(to bottom, rgba(215, 235, 252, 0.9), rgba(140, 190, 235, 0.8))",
    titleText: "#000",
    body: "#fff",
    accent: "#5a8fc4",
    taskbar: "linear-gradient(to bottom, #5b7898, #1d3554)",
  },
  components: {
    CaptionGlyph: AeroCaptionGlyph,
    StartButton: Win7StartOrb,
  },
});
