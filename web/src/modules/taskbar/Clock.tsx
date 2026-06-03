/**
 * Clock — live clock displayed in the system tray area of the taskbar.
 * Updates every second using setInterval.
 */

import { useState, useEffect } from "react";

function getTimeString(): string {
  return new Date().toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function Clock() {
  const [time, setTime] = useState(getTimeString);

  useEffect(() => {
    const id = setInterval(() => setTime(getTimeString()), 1000);
    return () => clearInterval(id);
  }, []);

  return (
    <div id="taskbar-clock" className="h-full px-3 flex items-center justify-center text-[12px] select-none min-w-[70px] whitespace-nowrap" aria-label="System clock" aria-live="polite">
      {time}
    </div>
  );
}
