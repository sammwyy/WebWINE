import { createContext, useContext } from "react";

export interface WindowHandle {
  id: string;
  close: () => void;
  focus: () => void;
  minimize: () => void;
  toggleMaximize: () => void;
  setTitle: (title: string) => void;
}

export const WindowContext = createContext<WindowHandle | null>(null);

/** Access the handle of the window this component is rendered in. */
export function useWindow(): WindowHandle {
  const ctx = useContext(WindowContext);
  if (!ctx) throw new Error("useWindow must be used inside an AppWindow");
  return ctx;
}

/** Like `useWindow`, but returns null outside of a window (shared components). */
export function useOptionalWindow(): WindowHandle | null {
  return useContext(WindowContext);
}
