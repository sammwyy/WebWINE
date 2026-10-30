import { setTheme, useCurrentTheme, useThemes } from "@/shared/lib/theme-registry";
import { ThemeThumbnail } from "./ThemeThumbnail";

interface ThemeSwitcherProps {
  /** controlled selection (e.g. a dialog with OK/Apply); defaults to applying immediately */
  value?: string;
  onChange?: (id: string) => void;
  /** double-click / activate a theme */
  onActivate?: (id: string) => void;
  className?: string;
}

/** Grid of theme thumbnails. Without `onChange` it switches the theme on click. */
export function ThemeSwitcher({ value, onChange, onActivate, className = "" }: ThemeSwitcherProps) {
  const themes = useThemes();
  const current = useCurrentTheme();
  const selected = value ?? current?.id;

  return (
    <div className={`ww-theme-switcher ${className}`} role="listbox" aria-label="Themes">
      {themes.map((theme) => (
        <button
          key={theme.id}
          type="button"
          role="option"
          aria-selected={theme.id === selected}
          className={`ww-theme-chip ${theme.id === selected ? "selected" : ""}`}
          title={theme.description}
          onClick={() => (onChange ? onChange(theme.id) : setTheme(theme.id))}
          onDoubleClick={() => onActivate?.(theme.id)}
        >
          <ThemeThumbnail theme={theme} />
          <span>{theme.name}</span>
        </button>
      ))}
    </div>
  );
}
