import "../aero/aero.css";
import "../aero/shell.css";
import "./vista.css";
import { registerTheme } from "@/shared/lib/theme-registry";
import { AeroCaptionGlyph } from "../aero/glyphs";
import { VistaStartOrb } from "../aero/orb";

export const vistaTheme = registerTheme({
  id: "vista",
  name: "Windows Vista (Aero)",
  description: "Aero glass: translucent blurred frames, glossy controls and the black glass orb.",
  inherits: ["aero"],
  startMenu: "classic",
  preferredWallpaper: "vista",
  preferredSoundPack: "vista",
  variants: { mediaPlayer: "wmp11" },
  preview: {
    desktop: "radial-gradient(ellipse at 60% 40%, #2fae8a 0%, #1a8a9a 30%, #1256a0 60%, #0a3070 100%)",
    frame: "linear-gradient(to bottom, rgba(200, 225, 250, 0.85), rgba(110, 160, 215, 0.75))",
    titleText: "#000",
    body: "#fff",
    accent: "#3c7fb1",
    taskbar: "linear-gradient(to bottom, #4a5a6c, #0c121c)",
  },
  components: {
    CaptionGlyph: AeroCaptionGlyph,
    StartButton: VistaStartOrb,
  },
});
