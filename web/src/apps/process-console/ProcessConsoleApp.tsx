import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  useCallback,
} from "react";
import { useWindowStore } from "@/state/windowStore";
import type { RuntimeBridge } from "@/core/bridge/runtime-bridge";
import type { LogEvent, UiEvent } from "@/core/wasm/worker";
import { handleUiEvents } from "../guest-window/GuestWindowApp";
import { basename } from "@/shared/lib/utils";

import { resolveIcon } from "@/shared/lib/icons/icon-resolver";

export interface ConsoleOptions {
  debug?: boolean;
  attachPid?: number;
  /** Extra command-line arguments appended after argv[0] (the image path). */
  args?: string;
}

export async function openProcessConsole(
  path: string,
  runtime: RuntimeBridge,
  opts: ConsoleOptions = {},
) {
  const name = basename(path);

  const resolved = await resolveIcon(
    { name, path, kind: "file", size: 0 },
    runtime,
  );
  const icon =
    resolved?.src ||
    `${import.meta.env.BASE_URL}theme/icons/shell/default_executable.webp`;

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
        winId={() => winId}
      />
    ),
  });
}

export async function launchProcessHidden(
  path: string,
  runtime: RuntimeBridge,
  opts: Pick<ConsoleOptions, "args"> = {},
): Promise<void> {
  const launched = opts.args
    ? await runtime.launchProcessWithArgs(path, opts.args)
    : await runtime.launchProcess(path);

  runtime.onProcessOutput(launched.pid, {
    stdout: () => {},
    stderr: () => {},
    ui: (events: UiEvent[]) => {
      handleUiEvents(launched.pid, events, runtime);
    },
    exited: () => {
      window.dispatchEvent(new CustomEvent("webwine:fs-changed"));
    },
    crashed: () => {
      window.dispatchEvent(new CustomEvent("webwine:fs-changed"));
    },
  });

  runtime.runProcess(launched.pid);
}

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

function countNewLines(text: string): number {
  let n = 0;
  for (let i = 0; i < text.length; i++) {
    if (text[i] === "\n") n++;
  }
  return n;
}

function isAtBottom(el: HTMLElement, thresholdPx = 40): boolean {
  return el.scrollHeight - el.scrollTop - el.clientHeight <= thresholdPx;
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
  const [stateText, setStateText] = useState("starting");
  const [pid, setPid] = useState<number | null>(null);
  const [running, setRunning] = useState(false);
  const [exited, setExited] = useState(false);
  const [pendingLines, setPendingLines] = useState(0);
  /** Forces a re-render of the banner without thrashing on every line. */
  const [bannerTick, setBannerTick] = useState(0);

  const termRef = useRef<HTMLDivElement>(null);
  /** Stick to bottom until the user scrolls away. Starts true so launch output follows. */
  const stickToBottomRef = useRef(true);
  /** Ignore scroll events caused by our own scrollTop writes. */
  const ignoreScrollRef = useRef(false);
  const pendingLinesRef = useRef(0);
  const bannerRafRef = useRef(0);

  const applyScrollBottom = useCallback(() => {
    const el = termRef.current;
    if (!el) return;
    ignoreScrollRef.current = true;
    el.scrollTop = el.scrollHeight;
    // Second pass after layout (rapid text can grow after the first write).
    requestAnimationFrame(() => {
      const node = termRef.current;
      if (node && stickToBottomRef.current) {
        node.scrollTop = node.scrollHeight;
      }
      // Keep ignoring until after the browser has delivered any scroll events.
      requestAnimationFrame(() => {
        ignoreScrollRef.current = false;
      });
    });
  }, []);

  const pinToBottom = useCallback(() => {
    stickToBottomRef.current = true;
    pendingLinesRef.current = 0;
    setPendingLines(0);
    applyScrollBottom();
  }, [applyScrollBottom]);

  // After paint with new text, stick to bottom if enabled.
  useLayoutEffect(() => {
    if (stickToBottomRef.current) {
      applyScrollBottom();
    }
  }, [spans, applyScrollBottom]);

  // User scroll tracking — never treat programmatic scrolls as "user left bottom".
  useEffect(() => {
    const el = termRef.current;
    if (!el) return;

    const onScroll = () => {
      if (ignoreScrollRef.current) return;
      if (isAtBottom(el)) {
        stickToBottomRef.current = true;
        if (pendingLinesRef.current !== 0) {
          pendingLinesRef.current = 0;
          setPendingLines(0);
        }
      } else {
        stickToBottomRef.current = false;
      }
    };

    el.addEventListener("scroll", onScroll, { passive: true });
    return () => el.removeEventListener("scroll", onScroll);
  }, []);

  // Title sync (only when pid/state change — not every byte of output).
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

  const bumpPending = useCallback((lineDelta: number) => {
    if (lineDelta <= 0) return;
    pendingLinesRef.current += lineDelta;
    // Coalesce banner updates to one per frame so rapid stdout doesn't thrash React.
    if (bannerRafRef.current === 0) {
      bannerRafRef.current = requestAnimationFrame(() => {
        bannerRafRef.current = 0;
        setPendingLines(pendingLinesRef.current);
        setBannerTick((t) => t + 1);
      });
    }
  }, []);

  const appendSpans = useCallback(
    (newSpans: TermSpan[], lineDelta: number) => {
      setSpans((s) => [...s, ...newSpans]);
      if (!stickToBottomRef.current) {
        bumpPending(lineDelta);
      }
    },
    [bumpPending],
  );

  const write = useCallback(
    (text: string, cls?: string) => {
      appendSpans([{ text, cls }], countNewLines(text));
    },
    [appendSpans],
  );

  const writeLog = useCallback(
    (ev: LogEvent) => {
      const levelCls =
        ev.level === "error"
          ? "text-[#c50f1f]"
          : ev.level === "warn"
            ? "text-[#f9f1a5]"
            : "text-[#3b78ff]";
      appendSpans(
        [
          { text: `[${ev.target}] `, cls: `${levelCls} font-bold` },
          { text: `${ev.message}\n`, cls: levelCls },
        ],
        1,
      );
    },
    [appendSpans],
  );

  useEffect(() => {
    let active = true;
    // Ensure stick is on at launch so the first flood of CRT output follows.
    stickToBottomRef.current = true;

    const ready =
      opts.attachPid !== undefined
        ? Promise.resolve({
            pid: opts.attachPid,
            launchLogs: [] as LogEvent[],
            attached: true,
          })
        : opts.args
          ? runtime
              .launchProcessWithArgs(path, opts.args)
              .then((r) => ({ ...r, attached: false }))
          : runtime
              .launchProcess(path)
              .then((r) => ({ ...r, attached: false }));

    ready
      .then(({ pid: newPid, launchLogs }) => {
        if (!active) return;

        setPid(newPid);
        setRunning(true);
        setStateText("running");
        stickToBottomRef.current = true;

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
              appendSpans(newSpans, countNewLines(text));
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
      if (bannerRafRef.current) {
        cancelAnimationFrame(bannerRafRef.current);
        bannerRafRef.current = 0;
      }
    };
  }, [
    path,
    runtime,
    opts.attachPid,
    opts.args,
    debug,
    write,
    writeLog,
    appendSpans,
  ]);

  const onKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (!running || pid === null) return;

      if (e.key === "Enter") {
        e.preventDefault();
        // Typing should follow output if we were stuck to bottom.
        write("\n", debug ? "text-[#a6e3a1]" : undefined);
        runtime.writeStdin(pid, "\r\n");
      } else if (e.key === "Backspace") {
        e.preventDefault();
        runtime.writeStdin(pid, "\x08");
        setSpans((prev) => {
          const newSpans = [...prev];
          for (let i = newSpans.length - 1; i >= 0; i--) {
            if (newSpans[i].text.length > 0) {
              newSpans[i] = {
                ...newSpans[i],
                text: newSpans[i].text.slice(0, -1),
              };
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

  const shownPending = pendingLines;
  const pendingLabel =
    shownPending === 1
      ? "Go to bottom (1 new line)"
      : `Go to bottom (${shownPending} new lines)`;

  return (
    <div className="relative bg-[#0c0c0c] text-[#cccccc] font-['Consolas','Lucida_Console',monospace] text-[14px] flex flex-col h-full min-h-0">
      <div
        className="flex-1 min-h-0 overflow-auto p-2 flex flex-col whitespace-pre-wrap break-all outline-none"
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
          {!exited && (
            <span className="animate-[blink_1s_step-end_infinite]">█</span>
          )}
        </span>
      </div>

      {shownPending > 0 && (
        <button
          type="button"
          key={bannerTick}
          onMouseDown={(e) => {
            // Prevent focus steal / scroll races before click.
            e.preventDefault();
            e.stopPropagation();
            pinToBottom();
          }}
          onClick={(e) => {
            e.preventDefault();
            e.stopPropagation();
            pinToBottom();
          }}
          className="absolute bottom-3 left-1/2 -translate-x-1/2 z-10 px-3 py-1.5 text-[12px] font-sans font-medium text-white bg-[#0078d7] border border-[#0078d7] rounded-sm shadow-[0_4px_16px_rgba(0,0,0,0.45)] hover:bg-[#006cc1] hover:border-[#006cc1] cursor-pointer whitespace-nowrap"
        >
          {pendingLabel}
        </button>
      )}
    </div>
  );
}
