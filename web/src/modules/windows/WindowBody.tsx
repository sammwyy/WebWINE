import type { HTMLAttributes } from "react";

interface WindowBodyProps extends HTMLAttributes<HTMLDivElement> {
  /** `white` (documents, lists), `face` (dialogs) or `none` (the content paints itself) */
  surface?: "white" | "face" | "none";
  /** make the body itself scrollable instead of a flex column */
  scroll?: boolean;
}

/** Opaque client surface with the theme's "client edge" border. */
export function WindowBody({ surface = "white", scroll = false, className = "", ...props }: WindowBodyProps) {
  return (
    <div
      className={`ww-body ${surface === "white" ? "" : surface} ${scroll ? "scroll" : ""} ${className}`}
      {...props}
    />
  );
}

/** A region painted directly on the frame/glass (toolbars, address bar, dialog footers...). */
export function GlassPanel({ className = "", ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={`ww-glass-panel ${className}`} {...props} />;
}
