import { useState } from "react";
import { useWindowStore } from "../../stores/useWindowStore.js";
import { useThemeStore } from "../../stores/useThemeStore.js";
import { THEMES } from "../../lib/themes.js";

export function openThemeSwitcher() {
  const store = useWindowStore.getState();
  const theme = useThemeStore.getState().theme;
  // Prevent multiple instances
  const existing = store.windows.find((w) => w.title === "Themes");
  if (existing) {
    store.restoreWindow(existing.id);
    store.focusWindow(existing.id);
    return;
  }

  store.openWindow({
    title: "Themes",
    icon: `/themes/${theme}/icons/apps/themes.webp`,
    width: 480,
    height: 380,
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
            <ThemePreview themeId={t.id} />
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

function ThemePreview({ themeId }: { themeId: string }) {
  const [error, setError] = useState(false);

  if (error) {
    return (
      <span
        className={`theme-preview theme-preview-${themeId}`}
        aria-hidden="true"
      />
    );
  }

  return (
    <img
      src={`/themes/${themeId}/icon.webp`}
      className={`theme-preview theme-preview-img theme-preview-${themeId}`}
      alt={`${themeId} preview`}
      draggable={false}
      onError={() => setError(true)}
    />
  );
}
