import { createElement, useState } from "react";
import type { ReactNode } from "react";
import { ChevronDownGlyph, ChevronRightGlyph, SystemIcon } from "../icons";
import { Icon } from "./Icon";
import { useThemed } from "./themed";

export interface Crumb {
  label: ReactNode;
  /** icon of this location (shown by themes with a location drop-down) */
  icon?: ReactNode;
  onClick?: () => void;
}

export interface AddressBarProps {
  icon?: ReactNode;
  path: Crumb[];
  onRefresh?: () => void;
  /** text shown when the address is edited (click the empty area of the bar) */
  editValue?: string;
  /** makes the address editable; called with the typed location on Enter */
  onNavigate?: (location: string) => void;
  className?: string;
}

/** Breadcrumb address bar with a refresh button (meant for a GlassPanel). */
function DefaultAddressBar({ icon, path, onRefresh, editValue = "", onNavigate, className = "" }: AddressBarProps) {
  const [editing, setEditing] = useState<string | null>(null);

  return (
    <>
      <div className={`ww-addressbar ${className}`}>
        <span className="ww-addressbar-icon">
          <Icon icon={path[path.length - 1]?.icon ?? icon} size={16} />
        </span>
        {editing !== null ? (
          <input
            className="ww-addressbar-edit"
            value={editing}
            autoFocus
            spellCheck={false}
            onFocus={(e) => e.currentTarget.select()}
            onChange={(e) => setEditing(e.target.value)}
            onBlur={() => setEditing(null)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                onNavigate?.(editing);
                setEditing(null);
              }
              if (e.key === "Escape") setEditing(null);
            }}
          />
        ) : (
          <>
            <span className="ww-crumb">
              <span className="chevron">
                <ChevronRightGlyph />
              </span>
            </span>
            <div
              className="ww-crumbs"
              onClick={(e) => {
                if (onNavigate && e.target === e.currentTarget) setEditing(editValue);
              }}
            >
              {path.map((crumb, i) => (
                <span key={i} className="ww-crumb">
                  <span onClick={crumb.onClick}>{crumb.label}</span>
                  <span className="chevron">
                    <ChevronRightGlyph />
                  </span>
                </span>
              ))}
            </div>
          </>
        )}
        <span className="ww-addressbar-dropdown" onClick={() => onNavigate && setEditing(editValue)}>
          <ChevronDownGlyph />
        </span>
      </div>
      <button className="ww-glass-btn" title="Refresh" onClick={onRefresh}>
        <SystemIcon name="view-refresh" size={16} />
      </button>
    </>
  );
}

/** Themeable: themes can replace it via `components.ui.AddressBar`. */
export function AddressBar(props: AddressBarProps) {
  return createElement(useThemed("AddressBar", DefaultAddressBar), props);
}
