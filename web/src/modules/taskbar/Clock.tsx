/**
 * Clock — live clock displayed in the system tray area of the taskbar.
 * Updates every second using setInterval.
 */

import { useState, useEffect } from "react";
import { useSettingsStore } from "@/state/settingsStore";

function getTimeString(seconds: boolean): string {
  return new Date().toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    ...(seconds ? { second: "2-digit" } : {}),
  });
}

export function Clock() {
  const seconds = useSettingsStore((s) => s.clockSeconds);
  const [time, setTime] = useState(() => getTimeString(seconds));

  useEffect(() => {
    setTime(getTimeString(seconds));
    const id = setInterval(() => setTime(getTimeString(seconds)), 1000);
    return () => clearInterval(id);
  }, [seconds]);

  return (
    <div
      id="taskbar-clock"
      className="ww-clock whitespace-nowrap"
      title={new Date().toLocaleDateString([], { dateStyle: "full" })}
      aria-label="System clock"
      aria-live="polite"
    >
      {time}
    </div>
  );
}
