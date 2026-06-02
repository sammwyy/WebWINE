import { createWindow } from "./manager.js";
import type { RuntimeBridge } from "../runtime-bridge.js";
import type { LogEvent, UiEvent } from "../worker.js";
import { handleUiEvents } from "./guest-windows.js";

interface ConsoleOptions {
  debug?: boolean;
  /** Attach to an already-launched pid (e.g. a CreateProcess child) instead of launching. */
  attachPid?: number;
}

export function openProcessConsole(path: string, runtime: RuntimeBridge, opts: ConsoleOptions = {}) {
  const debug = opts.debug ?? false;
  const fileName = path.split("\\").pop() ?? path;

  const { body, setTitle } = createWindow({
    title: `${fileName}`,
    icon: debug ? "🐞" : "🖥️",
    width: 660,
    height: 420,
  });

  // cmd-style terminal: a single focusable monospace surface. Output and the
  // currently-typed stdin line live in the same stream (no input box).
  body.className += " term-body";
  const term = document.createElement("div");
  term.className = "term";
  term.tabIndex = 0;

  const output = document.createElement("span");
  output.className = "term-output";
  const inputLine = document.createElement("span");
  inputLine.className = "term-input";
  const cursor = document.createElement("span");
  cursor.className = "term-cursor";
  cursor.textContent = "█";

  term.append(output, inputLine, cursor);
  body.append(term);

  // title state 
  function title(state: string) {
    const tag = debug ? " | debug" : "";
    setTitle(`${fileName} | pid ${pid ?? "?"} | ${state}${tag}`);
  }

  // output helpers 
  let pid: number | null = null;
  let running = false;

  function write(text: string, cls?: string) {
    const span = document.createElement("span");
    if (cls) span.className = cls;
    span.textContent = text;
    output.append(span);
    term.scrollTop = term.scrollHeight;
  }

  function writeLog(ev: LogEvent) {
    // [target] message — coloured by level
    write(`[${ev.target}] `, `term-log term-log-${ev.level} term-log-target`);
    write(`${ev.message}\n`, `term-log term-log-${ev.level}`);
  }

  // inline stdin 
  let lineBuf = "";
  function renderInput() {
    inputLine.textContent = lineBuf;
  }

  term.addEventListener("keydown", (e) => {
    if (!running || pid === null) return;
    if (e.key === "Enter") {
      e.preventDefault();
      const line = lineBuf + "\n";
      write(lineBuf + "\n", debug ? "term-stdin" : undefined);
      lineBuf = "";
      renderInput();
      runtime.writeStdin(pid, line);
    } else if (e.key === "Backspace") {
      e.preventDefault();
      lineBuf = lineBuf.slice(0, -1);
      renderInput();
    } else if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
      e.preventDefault();
      lineBuf += e.key;
      renderInput();
    }
  });

  term.addEventListener("mousedown", () => term.focus());

  // launch (or attach) + wire 
  title("starting");

  // Attaching to an existing child resolves immediately; launching starts a new process.
  const ready: Promise<{ pid: number; launchLogs: LogEvent[]; attached: boolean }> =
    opts.attachPid !== undefined
      ? Promise.resolve({ pid: opts.attachPid, launchLogs: [], attached: true })
      : runtime.launchProcess(path).then((r) => ({ ...r, attached: false }));

  ready.then(({ pid: newPid, launchLogs, attached }) => {
    pid = newPid;
    running = true;
    title("running");
    term.focus();

    if (debug) {
      for (const ev of launchLogs) writeLog(ev);
    }

    runtime.onProcessOutput(newPid, {
      stdout: (text) => {
        if (debug) {
          for (const line of splitKeepLast(text)) {
            if (line.content) write("[stdout] ", "term-log term-log-info term-log-target");
            write(line.content + (line.nl ? "\n" : ""), "term-stdout-dbg");
          }
        } else {
          write(text);
        }
      },
      stderr: (text) => write(text, debug ? "term-stderr-dbg" : "term-stderr"),
      ui: (events: UiEvent[]) => handleUiEvents(newPid, events, runtime),
      log: debug ? (events) => { for (const ev of events) writeLog(ev); } : undefined,
      exited: (code) => {
        running = false;
        title(`exited (${code})`);
        write(`\n[process exited with code ${code}]\n`, "term-exit");
        cursor.style.display = "none";
        // guest may have created files — refresh desktop/explorers
        window.dispatchEvent(new CustomEvent("webwine:fs-changed"));
      },
      crashed: (reason) => {
        running = false;
        title("crashed");
        write(`\n[process crashed: ${reason}]\n`, "term-crash");
        cursor.style.display = "none";
        window.dispatchEvent(new CustomEvent("webwine:fs-changed"));
      },
    });

    // Start the run loop. A spawned child was loaded by the VM but isn't being
    // sliced yet, so it also needs its own run loop here.
    void attached;
    runtime.runProcess(pid);
  }).catch((err) => {
    title("error");
    write(`\n[failed to launch: ${err}]\n`, "term-crash");
  });
}

// Split text into lines, marking which ended with a newline — so the debug
// "[stdout]" prefix is only emitted once per actual line.
function splitKeepLast(text: string): { content: string; nl: boolean }[] {
  const parts = text.split("\n");
  const out: { content: string; nl: boolean }[] = [];
  for (let i = 0; i < parts.length; i++) {
    const isLast = i === parts.length - 1;
    if (isLast && parts[i] === "") continue;
    out.push({ content: parts[i], nl: !isLast });
  }
  return out;
}
