/**
 * Process console — guest stdout/stderr/stdin.
 *
 * Terminal *content* is written straight to the DOM (batched per frame). React
 * state is only used for chrome that actually needs it (title bits, stick-to-
 * bottom banner, exit state). That keeps the window manager / drag free even
 * when a process floods thousands of lines.
 */

import {
  useEffect,
  useRef,
  useState,
  useCallback,
  memo,
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

/** Soft cap: drop oldest text when buffer exceeds this many characters. */
const MAX_TERM_CHARS = 400_000;
/** Hard prune target after overflow (keep recent tail). */
const TRIM_TERM_CHARS = 300_000;

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

type PendingChunk = { text: string; cls?: string };

/**
 * Imperative terminal buffer: queues writes and flushes once per animation
 * frame into a single <pre> via DocumentFragment. Zero React involvement.
 */
class TermBuffer {
  private readonly host: HTMLElement;
  private readonly body: HTMLElement;
  private readonly caret: HTMLElement;
  private queue: PendingChunk[] = [];
  private raf = 0;
  private charCount = 0;
  stickToBottom = true;
  private ignoreScroll = false;
  private pendingLines = 0;
  private onPendingChange: ((n: number) => void) | null = null;
  private bannerRaf = 0;

  constructor(host: HTMLElement) {
    this.host = host;
    host.replaceChildren();

    this.body = document.createElement("pre");
    this.body.className =
      "m-0 p-0 font-inherit text-inherit whitespace-pre-wrap break-all";
    this.body.setAttribute("aria-live", "off");

    this.caret = document.createElement("span");
    this.caret.className = "animate-[blink_1s_step-end_infinite]";
    this.caret.textContent = "█";
    this.caret.hidden = false;

    // Scroll container is `host`; body+caret live inside.
    const wrap = document.createElement("div");
    wrap.append(this.body, this.caret);
    host.append(wrap);

    host.addEventListener(
      "scroll",
      () => {
        if (this.ignoreScroll) return;
        if (isAtBottom(host)) {
          this.stickToBottom = true;
          this.setPending(0);
        } else {
          this.stickToBottom = false;
        }
      },
      { passive: true },
    );
  }

  setPendingListener(fn: (n: number) => void) {
    this.onPendingChange = fn;
  }

  setCaretVisible(visible: boolean) {
    this.caret.hidden = !visible;
  }

  private setPending(n: number) {
    this.pendingLines = n;
    if (this.bannerRaf) return;
    this.bannerRaf = requestAnimationFrame(() => {
      this.bannerRaf = 0;
      this.onPendingChange?.(this.pendingLines);
    });
  }

  write(text: string, cls?: string) {
    if (!text) return;
    this.queue.push({ text, cls });
    if (!this.stickToBottom) {
      this.setPending(this.pendingLines + countNewLines(text));
    }
    if (!this.raf) {
      this.raf = requestAnimationFrame(() => this.flush());
    }
  }

  /** Immediate flush (e.g. before backspace). */
  flushSync() {
    if (this.raf) {
      cancelAnimationFrame(this.raf);
      this.raf = 0;
    }
    this.flush();
  }

  private flush() {
    this.raf = 0;
    if (this.queue.length === 0) return;

    const chunks = this.queue;
    this.queue = [];

    const frag = document.createDocumentFragment();
    for (const { text, cls } of chunks) {
      if (cls) {
        const span = document.createElement("span");
        span.className = cls;
        span.textContent = text;
        frag.append(span);
      } else {
        frag.append(document.createTextNode(text));
      }
      this.charCount += text.length;
    }
    this.body.append(frag);

    if (this.charCount > MAX_TERM_CHARS) {
      this.trimOld();
    }

    if (this.stickToBottom) {
      this.scrollToBottom();
    }
  }

  private trimOld() {
    // Drop whole leading nodes until under TRIM_TERM_CHARS.
    while (
      this.charCount > TRIM_TERM_CHARS &&
      this.body.firstChild
    ) {
      const node = this.body.firstChild;
      const len =
        node.nodeType === Node.TEXT_NODE
          ? (node.textContent?.length ?? 0)
          : (node.textContent?.length ?? 0);
      this.body.removeChild(node);
      this.charCount = Math.max(0, this.charCount - len);
    }
  }

  scrollToBottom() {
    this.stickToBottom = true;
    this.setPending(0);
    this.ignoreScroll = true;
    this.host.scrollTop = this.host.scrollHeight;
    requestAnimationFrame(() => {
      this.host.scrollTop = this.host.scrollHeight;
      requestAnimationFrame(() => {
        this.ignoreScroll = false;
      });
    });
  }

  /** Delete one character from the end (local echo backspace). */
  backspace() {
    this.flushSync();
    // Walk backwards through last text node / span.
    let node: ChildNode | null = this.body.lastChild;
    while (node) {
      if (node.nodeType === Node.TEXT_NODE) {
        const t = node.textContent ?? "";
        if (t.length > 0) {
          node.textContent = t.slice(0, -1);
          this.charCount = Math.max(0, this.charCount - 1);
          if (node.textContent.length === 0) node.parentNode?.removeChild(node);
          return;
        }
      } else if (node.nodeType === Node.ELEMENT_NODE) {
        const el = node as HTMLElement;
        const t = el.textContent ?? "";
        if (t.length > 0) {
          el.textContent = t.slice(0, -1);
          this.charCount = Math.max(0, this.charCount - 1);
          if (!el.textContent) el.remove();
          return;
        }
      }
      const prev = node.previousSibling;
      node.parentNode?.removeChild(node);
      node = prev;
    }
  }

  dispose() {
    if (this.raf) cancelAnimationFrame(this.raf);
    if (this.bannerRaf) cancelAnimationFrame(this.bannerRaf);
    this.queue = [];
  }
}

// memo: parent WindowFrame may re-render on focus/title; console chrome stays put.
const ProcessConsoleApp = memo(function ProcessConsoleApp({
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

  const [stateText, setStateText] = useState("starting");
  const [pid, setPid] = useState<number | null>(null);
  const [pendingLines, setPendingLines] = useState(0);

  const scrollRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<TermBuffer | null>(null);
  const pidRef = useRef<number | null>(null);
  const runningRef = useRef(false);

  // Mount imperative terminal once.
  useEffect(() => {
    const host = scrollRef.current;
    if (!host) return;
    const term = new TermBuffer(host);
    term.setPendingListener(setPendingLines);
    termRef.current = term;
    return () => {
      term.dispose();
      termRef.current = null;
    };
  }, []);

  const write = useCallback((text: string, cls?: string) => {
    termRef.current?.write(text, cls);
  }, []);

  const writeLog = useCallback(
    (ev: LogEvent) => {
      const levelCls =
        ev.level === "error"
          ? "text-[#c50f1f]"
          : ev.level === "warn"
            ? "text-[#f9f1a5]"
            : "text-[#3b78ff]";
      termRef.current?.write(`[${ev.target}] `, `${levelCls} font-bold`);
      termRef.current?.write(`${ev.message}\n`, levelCls);
    },
    [],
  );

  // Title only when pid/state change — never on stdout.
  useEffect(() => {
    const id = winId();
    if (!id) return;
    const tag = debug ? " | debug" : "";
    useWindowStore
      .getState()
      .setTitle(id, `${fileName} | pid ${pid ?? "?"} | ${stateText}${tag}`);
  }, [pid, stateText, debug, fileName, winId]);

  useEffect(() => {
    let active = true;
    if (termRef.current) termRef.current.stickToBottom = true;

    const ready =
      opts.attachPid !== undefined
        ? Promise.resolve({
            pid: opts.attachPid,
            launchLogs: [] as LogEvent[],
          })
        : opts.args
          ? runtime.launchProcessWithArgs(path, opts.args)
          : runtime.launchProcess(path);

    ready
      .then(({ pid: newPid, launchLogs }) => {
        if (!active) return;

        pidRef.current = newPid;
        runningRef.current = true;
        setPid(newPid);
        setStateText("running");
        if (termRef.current) {
          termRef.current.stickToBottom = true;
          termRef.current.setCaretVisible(true);
        }

        if (debug && launchLogs) {
          for (const ev of launchLogs) writeLog(ev);
        }

        runtime.onProcessOutput(newPid, {
          stdout: (text) => {
            if (!active) return;
            if (debug) {
              // Prefix each logical line once in debug mode without React.
              const parts = text.split("\n");
              for (let i = 0; i < parts.length; i++) {
                const isLast = i === parts.length - 1;
                if (isLast && parts[i] === "") continue;
                if (parts[i]) {
                  termRef.current?.write(
                    "[stdout] ",
                    "text-[#3b78ff] font-bold",
                  );
                }
                termRef.current?.write(
                  parts[i] + (isLast ? "" : "\n"),
                  "text-[#cccccc]",
                );
              }
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
                for (const ev of events) writeLog(ev);
              }
            : undefined,
          exited: (code) => {
            if (!active) return;
            runningRef.current = false;
            setStateText(`exited (${code})`);
            termRef.current?.setCaretVisible(false);
            write(
              `\n[process exited with code ${code}]\n`,
              "text-[#89b4fa]",
            );
            window.dispatchEvent(new CustomEvent("webwine:fs-changed"));
          },
          crashed: (reason) => {
            if (!active) return;
            runningRef.current = false;
            setStateText("crashed");
            termRef.current?.setCaretVisible(false);
            write(`\n[process crashed: ${reason}]\n`, "text-[#c50f1f]");
            window.dispatchEvent(new CustomEvent("webwine:fs-changed"));
          },
        });

        runtime.runProcess(newPid);
      })
      .catch((err) => {
        if (!active) return;
        setStateText("error");
        termRef.current?.setCaretVisible(false);
        write(`\n[failed to launch: ${err}]\n`, "text-[#c50f1f]");
      });

    return () => {
      active = false;
    };
  }, [path, runtime, opts.attachPid, opts.args, debug, write, writeLog]);

  const pinToBottom = useCallback(() => {
    termRef.current?.scrollToBottom();
  }, []);

  const onKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      const p = pidRef.current;
      if (!runningRef.current || p === null) return;

      if (e.key === "Enter") {
        e.preventDefault();
        write("\n", debug ? "text-[#a6e3a1]" : undefined);
        runtime.writeStdin(p, "\r\n");
      } else if (e.key === "Backspace") {
        e.preventDefault();
        runtime.writeStdin(p, "\x08");
        termRef.current?.backspace();
      } else if (e.key === "Tab") {
        e.preventDefault();
        write("\t", debug ? "text-[#a6e3a1]" : undefined);
        runtime.writeStdin(p, "\t");
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        runtime.writeStdin(p, "\x1b[A");
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        runtime.writeStdin(p, "\x1b[B");
      } else if (e.key === "ArrowRight") {
        e.preventDefault();
        runtime.writeStdin(p, "\x1b[C");
      } else if (e.key === "ArrowLeft") {
        e.preventDefault();
        runtime.writeStdin(p, "\x1b[D");
      } else if (e.key === "Escape") {
        e.preventDefault();
        runtime.writeStdin(p, "\x1b");
      } else if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
        e.preventDefault();
        write(e.key, debug ? "text-[#a6e3a1]" : undefined);
        runtime.writeStdin(p, e.key);
      }
    },
    [write, debug, runtime],
  );

  const pendingLabel =
    pendingLines === 1
      ? "Go to bottom (1 new line)"
      : `Go to bottom (${pendingLines} new lines)`;

  return (
    <div className="relative bg-[#0c0c0c] text-[#cccccc] font-['Consolas','Lucida_Console',monospace] text-[14px] flex flex-col h-full min-h-0 contain-strict">
      <div
        ref={scrollRef}
        className="flex-1 min-h-0 overflow-auto p-2 outline-none contain-content"
        tabIndex={0}
        onKeyDown={onKeyDown}
        onMouseDown={() => scrollRef.current?.focus()}
      />

      {pendingLines > 0 && (
        <button
          type="button"
          onMouseDown={(e) => {
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
});
