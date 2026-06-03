import { useEffect, useRef, useState, useCallback } from "react";
import { useWindowStore } from "../../state/windowStore";
import type { RuntimeBridge } from "../../core/bridge/runtime-bridge";
import type { LogEvent, UiEvent } from "../../core/wasm/worker";
import { handleUiEvents } from "../guest-window/GuestWindowApp";
import { basename } from "../../shared/lib/utils";

import { resolveIcon } from "../../shared/lib/icon-resolver";

interface ConsoleOptions {
  debug?: boolean;
  attachPid?: number;
}

export async function openProcessConsole(
  path: string,
  runtime: RuntimeBridge,
  opts: ConsoleOptions = {},
) {
  const name = basename(path);
  
  
  // Try to extract icon from PE, fallback to default_executable
  const resolved = await resolveIcon(
    { name, path, kind: "file", size: 0 },
    runtime
  );
  const icon = resolved?.src || `/theme/icons/shell/default_executable.webp`;

  let winId = "";
  winId = useWindowStore.getState().openWindow({
    title: name,
    icon,
    width: 640,
    height: 400,
    content: (
      <ProcessConsoleApp
        path={path}
        runtime={runtime}
        opts={opts}
        winId={() => winId} // lazy eval because winId is assigned after openWindow returns
      />
    ),
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

interface TermSpan {
  text: string;
  cls?: string;
}

function ProcessConsoleApp({
  path,
  runtime,
  opts,
  winId,
}: {
  path: string;
  runtime: RuntimeBridge;
  opts: ConsoleOptions;
  winId: () => string;
}) {
  const fileName = basename(path);
  const debug = opts.debug ?? false;

  const [spans, setSpans] = useState<TermSpan[]>([]);
  const [inputLine, setInputLine] = useState("");
  const [stateText, setStateText] = useState("starting");
  const [pid, setPid] = useState<number | null>(null);
  const [running, setRunning] = useState(false);
  const [exited, setExited] = useState(false);

  const termRef = useRef<HTMLDivElement>(null);

  // Auto-scroll term
  useEffect(() => {
    if (termRef.current) {
      termRef.current.scrollTop = termRef.current.scrollHeight;
    }
  }, [spans, inputLine]);

  // Title sync
  useEffect(() => {
    const id = winId();
    if (id) {
      const tag = debug ? " | debug" : "";
      useWindowStore
        .getState()
        .setTitle(
          id,
          `${fileName} | pid ${pid ?? "?"} | ${stateText}${tag}`,
        );
    }
  }, [pid, stateText, debug, fileName, winId]);

  const write = useCallback((text: string, cls?: string) => {
    setSpans((s) => [...s, { text, cls }]);
  }, []);

  const writeLog = useCallback((ev: LogEvent) => {
    const levelCls = ev.level === "error" ? "text-[#c50f1f]" : ev.level === "warn" ? "text-[#f9f1a5]" : "text-[#3b78ff]";
    setSpans((s) => [
      ...s,
      { text: `[${ev.target}] `, cls: `${levelCls} font-bold` },
      { text: `${ev.message}\n`, cls: levelCls },
    ]);
  }, []);

  // Launch process
  useEffect(() => {
    let active = true;

    const ready =
      opts.attachPid !== undefined
        ? Promise.resolve({
            pid: opts.attachPid,
            launchLogs: [] as LogEvent[],
            attached: true,
          })
        : runtime
            .launchProcess(path)
            .then((r) => ({ ...r, attached: false }));

    ready
      .then(({ pid: newPid, launchLogs }) => {
        if (!active) return;

        setPid(newPid);
        setRunning(true);
        setStateText("running");

        if (debug) {
          launchLogs.forEach(writeLog);
        }

        runtime.onProcessOutput(newPid, {
          stdout: (text) => {
            if (!active) return;
            if (debug) {
              const parts = splitKeepLast(text);
              const newSpans: TermSpan[] = [];
              for (const line of parts) {
                if (line.content) {
                  newSpans.push({
                    text: "[stdout] ",
                    cls: "text-[#3b78ff] font-bold",
                  });
                }
                newSpans.push({
                  text: line.content + (line.nl ? "\n" : ""),
                  cls: "text-[#cccccc]",
                });
              }
              setSpans((s) => [...s, ...newSpans]);
            } else {
              write(text);
            }
          },
          stderr: (text) => {
            if (active) write(text, "text-[#c50f1f]");
          },
          ui: (events: UiEvent[]) => {
            if (active) handleUiEvents(newPid, events, runtime);
          },
          log: debug
            ? (events) => {
                if (!active) return;
                events.forEach(writeLog);
              }
            : undefined,
          exited: (code) => {
            if (!active) return;
            setRunning(false);
            setExited(true);
            setStateText(`exited (${code})`);
            write(`\n[process exited with code ${code}]\n`, "text-[#89b4fa]");
            window.dispatchEvent(new CustomEvent("webwine:fs-changed"));
          },
          crashed: (reason) => {
            if (!active) return;
            setRunning(false);
            setExited(true);
            setStateText("crashed");
            write(`\n[process crashed: ${reason}]\n`, "text-[#c50f1f]");
            window.dispatchEvent(new CustomEvent("webwine:fs-changed"));
          },
        });

        runtime.runProcess(newPid);
      })
      .catch((err) => {
        if (!active) return;
        setStateText("error");
        write(`\n[failed to launch: ${err}]\n`, "text-[#c50f1f]");
      });

    return () => {
      active = false;
    };
  }, [path, runtime, opts.attachPid, debug, write, writeLog]);

  const onKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (!running || pid === null) return;

      if (e.key === "Enter") {
        e.preventDefault();
        write("\n", debug ? "text-[#a6e3a1]" : undefined);
        runtime.writeStdin(pid, "\n");
      } else if (e.key === "Backspace") {
        e.preventDefault();
        runtime.writeStdin(pid, "\x08");
        setSpans((prev) => {
          const newSpans = [...prev];
          for (let i = newSpans.length - 1; i >= 0; i--) {
             if (newSpans[i].text.length > 0) {
                 newSpans[i] = { ...newSpans[i], text: newSpans[i].text.slice(0, -1) };
                 break;
             }
          }
          return newSpans;
        });
      } else if (e.key === "Tab") {
        e.preventDefault();
        write("\t", debug ? "text-[#a6e3a1]" : undefined);
        runtime.writeStdin(pid, "\t");
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        runtime.writeStdin(pid, "\x1b[A");
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        runtime.writeStdin(pid, "\x1b[B");
      } else if (e.key === "ArrowRight") {
        e.preventDefault();
        runtime.writeStdin(pid, "\x1b[C");
      } else if (e.key === "ArrowLeft") {
        e.preventDefault();
        runtime.writeStdin(pid, "\x1b[D");
      } else if (e.key === "Escape") {
        e.preventDefault();
        runtime.writeStdin(pid, "\x1b");
      } else if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
        e.preventDefault();
        write(e.key, debug ? "text-[#a6e3a1]" : undefined);
        runtime.writeStdin(pid, e.key);
      }
    },
    [running, pid, write, debug, runtime],
  );

  return (
    <div className="bg-[#0c0c0c] text-[#cccccc] font-['Consolas','Lucida_Console',monospace] text-[14px] p-2 overflow-auto flex flex-col h-full">
      <div
        className="flex-1 flex flex-col whitespace-pre-wrap break-all outline-none"
        tabIndex={0}
        ref={termRef}
        onKeyDown={onKeyDown}
        onMouseDown={() => termRef.current?.focus()}
      >
        <span>
          {spans.map((s, i) => (
            <span key={i} className={s.cls}>
              {s.text}
            </span>
          ))}
          {!exited && <span className="animate-[blink_1s_step-end_infinite]">█</span>}
        </span>
      </div>
    </div>
  );
}
