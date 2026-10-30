/**
 * WindowFrame — pure, presentational themed window frame (`ww-window`).
 *
 * It knows nothing about the window store, so it can be used for previews,
 * dialogs or custom shells. The look comes entirely from the active theme
 * (src/themes/<id>/*.css). `AppWindow` connects it to the store.
 */

import type { HTMLAttributes, ReactNode, Ref } from "react";
import { TitleBar } from "./TitleBar";
import { WindowControls } from "./WindowControls";

export interface WindowFrameProps extends Omit<HTMLAttributes<HTMLDivElement>, "title"> {
  ref?: Ref<HTMLDivElement>;
  title: ReactNode;
  icon?: ReactNode;
  /** focused window: stronger glass tint, coloured close button… */
  active?: boolean;
  maximized?: boolean;
  minimizable?: boolean;
  maximizable?: boolean;
  /** dialog frame (themes may draw it slightly differently) */
  dialog?: boolean;
  /** no caption and no frame: the content draws its own chrome */
  frameless?: boolean;
  onMinimize?: () => void;
  onToggleMaximize?: () => void;
  onClose?: () => void;
  /** extra props for the caption (e.g. pointer handlers for dragging) */
  captionProps?: HTMLAttributes<HTMLDivElement>;
  /** rendered on top of the frame (e.g. resize handles) */
  overlay?: ReactNode;
  /** client area — sits on the frame; wrap content in <WindowBody> for an opaque surface */
  children?: ReactNode;
}

export function WindowFrame({
  title,
  icon,
  active = true,
  maximized = false,
  minimizable = true,
  maximizable = true,
  dialog = false,
  frameless = false,
  onMinimize,
  onToggleMaximize,
  onClose,
  captionProps,
  overlay,
  children,
  className = "",
  ...props
}: WindowFrameProps) {
  const classes = [
    "ww-window",
    active ? "active" : "inactive",
    maximized && "maximized",
    dialog && "dialog",
    frameless && "frameless",
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div className={classes} {...props}>
      {!frameless && (
        <TitleBar icon={icon} title={title} {...captionProps}>
          <WindowControls
            maximized={maximized}
            minimizable={minimizable}
            maximizable={maximizable}
            onMinimize={onMinimize}
            onToggleMaximize={onToggleMaximize}
            onClose={onClose}
          />
        </TitleBar>
      )}
      <div className="ww-window-client">{children}</div>
      {overlay}
    </div>
  );
}
