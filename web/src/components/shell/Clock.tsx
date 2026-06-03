/**
 * Clock — live clock displayed in the system tray area of the taskbar.
 * Updates every second using setInterval.
 */

import { useState, useEffect } from "react";
import styles from "./Taskbar.module.css";

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
    <div id="taskbar-clock" className={`${styles["taskbar-clock"]} taskbar-clock`} aria-label="System clock" aria-live="polite">
      {time}
    </div>
  );
}
