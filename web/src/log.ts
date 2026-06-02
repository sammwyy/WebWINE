import type { LogEvent } from "./worker.js";

export function log(target: string, message: string, level: LogEvent["level"] = "info") {
  appendLogs([{ level, target, message }]);
}

export function appendLogs(events: LogEvent[]) {
  const out = document.getElementById("log-output");
  if (!out) return;

  for (const ev of events) {
    const line = document.createElement("div");
    line.className = `log-line log-${ev.level}`;

    const tag = document.createElement("span");
    tag.className = "log-target";
    tag.textContent = `[${ev.target}]`;

    const msg = document.createElement("span");
    msg.className = "log-message";
    msg.textContent = ` ${ev.message}`;

    line.append(tag, msg);
    out.appendChild(line);
  }

  out.scrollTop = out.scrollHeight;
}
