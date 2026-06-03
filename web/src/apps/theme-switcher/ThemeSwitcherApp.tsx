import { useWindowStore } from "../../stores/useWindowStore.js";
import { THEMES, useThemeStore } from "../../stores/useThemeStore.js";

export function openThemeSwitcher() {
  useWindowStore.getState().openWindow({
    title: "Themes",
    icon: "🎨",
    width: 420,
    height: 346,
    resizable: false,
    content: <ThemeSwitcherApp />,
  });
}

function ThemeSwitcherApp() {
  const { theme, setTheme } = useThemeStore();

  return (
    <div className="themes-window-body" style={{ height: "100%" }}>
      <div className="themes-app">
        {THEMES.map((t) => (
          <label key={t.id} className="theme-option">
            <input
              type="radio"
              name="webwine-theme"
              value={t.id}
              checked={theme === t.id}
              onChange={(e) => {
                if (e.target.checked) setTheme(t.id);
              }}
            />
            <span
              className={`theme-preview theme-preview-${t.id}`}
              aria-hidden="true"
            />
            <span className="theme-option-text">
              <span className="theme-option-name">{t.name}</span>
              <span className="theme-option-description">{t.description}</span>
            </span>
          </label>
        ))}
      </div>
    </div>
  );
}
