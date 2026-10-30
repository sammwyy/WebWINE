import "./xp.css";
import "./shell.css";
import { registerTheme } from "@/shared/lib/theme-registry";
import { XpCaptionGlyph, XpStartButton } from "./components";
import { XpAddressBar, XpNavButtons, XpSearchButtons } from "./explorer";

export const xpTheme = registerTheme({
  id: "xp",
  name: "Windows XP (Luna)",
  description: "The blue Luna look: rounded blue title bars, Tahoma, the green start button and blue task panes.",
  startMenu: "classic",
  preferredWallpaper: "xp",
  preferredSoundPack: "xp",
  variants: { mediaPlayer: "wmp11" },
  preview: {
    desktop:
      "radial-gradient(ellipse 95% 48% at 32% 102%, #79b943 0%, #44901f 59%, transparent 60%), linear-gradient(to bottom, #1d64c8, #95c6f5 70%, #d2e8fb)",
    frame: "linear-gradient(to bottom, #0997ff, #0053ee 12%, #0050ee 60%, #06f)",
    titleText: "#fff",
    body: "#ece9d8",
    accent: "#0831d9",
    taskbar: "linear-gradient(to bottom, #3682e5, #2157d6 40%, #1941a5)",
  },
  components: {
    CaptionGlyph: XpCaptionGlyph,
    StartButton: XpStartButton,
    ui: {
      NavButtons: XpNavButtons,
      AddressBar: XpAddressBar,
      SearchBox: XpSearchButtons,
    },
  },
});
