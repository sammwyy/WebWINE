/**
 * Text Editor (Notepad) — themed menu bar with working commands:
 * New / Open / Save / Save As, clipboard, Find, word wrap, zoom, status bar.
 */

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useWindowStore } from "@/state/windowStore";
import type { RuntimeBridge } from "@/core/bridge/runtime-bridge";
import { basename } from "@/shared/lib/utils";
import { resolveIcon } from "@/shared/lib/icons/icon-resolver";
import { iconUrl } from "@/shared/lib/personalization";
import { MenuBar, type MenuBarItem } from "@/shared/components/MenuBar";
import { SEPARATOR } from "@/modules/desktop/ContextMenu";
import { StatusBar } from "@/shared/ui";
import { showFileDialog } from "../file-dialog/FileDialogApp";
import { showPrompt } from "../prompt/PromptDialog";
import { showMessageBox } from "../message-box/MessageBoxApp";

const APP_NAME = "WebWINE: Text Editor";
const TEXT_FILTER = "Text Documents (*.txt)|*.txt;*.log;*.ini;*.md|All Files (*.*)|*.*";
const DOCUMENTS = "C:\\Users\\guest\\Documents";

export async function openTextReader(path: string, runtime: RuntimeBridge) {
  const name = path ? basename(path) : "Untitled";
  const resolved = await resolveIcon({ name, path, kind: "file", size: 0 }, runtime);

  const id = useWindowStore.getState().openWindow({
    title: `${name} — ${APP_NAME}`,
    icon: resolved?.src || iconUrl("apps/notepad"),
    width: 720,
    height: 520,
    minWidth: 320,
    minHeight: 200,
    content: <div />,
  });
  useWindowStore.getState().setContent(id, <TextEditorApp initialPath={path} runtime={runtime} winId={id} />);
}

function TextEditorApp({ initialPath, runtime, winId }: { initialPath: string; runtime: RuntimeBridge; winId: string }) {
  const editorRef = useRef<HTMLTextAreaElement>(null);
  const [path, setPath] = useState(initialPath);
  const [content, setContent] = useState("");
  const [originalContent, setOriginalContent] = useState("");
  const [readonly, setReadonly] = useState(false);
  const [loading, setLoading] = useState(Boolean(initialPath));
  const [saving, setSaving] = useState(false);
  const [status, setStatus] = useState(initialPath ? "Loading…" : "New file");
  const [error, setError] = useState<string | null>(null);
  const [wordWrap, setWordWrap] = useState(false);
  const [zoom, setZoom] = useState(100);
  const [showStatus, setShowStatus] = useState(true);
  const [caret, setCaret] = useState({ line: 1, col: 1 });

  const name = path ? basename(path) : "Untitled";
  const dirty = content !== originalContent;

  const load = useCallback(
    async (target: string) => {
      setLoading(true);
      setError(null);
      setStatus("Loading…");
      try {
        const bytes = await runtime.readFile(target);
        try {
          const text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
          setContent(text);
          setOriginalContent(text);
          setReadonly(false);
          setStatus(`${bytes.length.toLocaleString()} bytes`);
        } catch {
          const dump = toHexDump(bytes);
          setContent(dump);
          setOriginalContent(dump);
          setReadonly(true);
          setStatus(`Binary file · ${bytes.length.toLocaleString()} bytes · read-only hexdump`);
        }
        setPath(target);
      } catch (err) {
        setError(`Could not open ${target}: ${err}`);
        setStatus("Failed to load");
      } finally {
        setLoading(false);
      }
    },
    [runtime],
  );

  useEffect(() => {
    if (initialPath) void load(initialPath);
  }, [initialPath, load]);

  useEffect(() => {
    useWindowStore.getState().setTitle(winId, `${dirty ? "*" : ""}${name} — ${APP_NAME}`);
  }, [dirty, name, winId]);

  /** Ask before discarding unsaved changes. Resolves true when it's OK to continue. */
  async function confirmDiscard(): Promise<boolean> {
    if (!dirty || readonly) return true;
    const answer = await showMessageBox({
      kind: "message_box",
      title: APP_NAME,
      text: `Do you want to save changes to ${name}?`,
      style: 0x03 | 0x30, // MB_YESNOCANCEL | MB_ICONEXCLAMATION
    });
    if (answer === "cancel") return false;
    if (answer === "yes") return saveTo(path || null);
    return true;
  }

  const writeFile = async (target: string) => {
    setSaving(true);
    setError(null);
    setStatus("Saving…");
    try {
      const bytes = new TextEncoder().encode(content);
      await runtime.mountFile(target, bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength));
      setPath(target);
      setOriginalContent(content);
      setStatus(`${bytes.length.toLocaleString()} bytes · saved`);
      window.dispatchEvent(new Event("webwine:fs-changed"));
      return true;
    } catch (err) {
      setError(`Save failed: ${err}`);
      setStatus("Save failed");
      return false;
    } finally {
      setSaving(false);
    }
  };

  /** Save to `target`, or ask for a location (Save As) when there is none. */
  async function saveTo(target: string | null): Promise<boolean> {
    if (readonly || loading || saving) return false;
    if (target) return writeFile(target);
    const chosen = await showFileDialog(
      {
        kind: "file_dialog",
        save: true,
        title: "Save As",
        filter: TEXT_FILTER,
        initial_dir: path ? path.slice(0, path.lastIndexOf("\\")) : DOCUMENTS,
        default_name: path ? basename(path) : "Untitled.txt",
      },
      runtime,
    );
    return chosen ? writeFile(chosen) : false;
  }

  const newFile = async () => {
    if (!(await confirmDiscard())) return;
    setPath("");
    setContent("");
    setOriginalContent("");
    setReadonly(false);
    setError(null);
    setStatus("New file");
  };

  const openFile = async () => {
    if (!(await confirmDiscard())) return;
    const chosen = await showFileDialog(
      {
        kind: "file_dialog",
        save: false,
        title: "Open",
        filter: TEXT_FILTER,
        initial_dir: path ? path.slice(0, path.lastIndexOf("\\")) : DOCUMENTS,
        default_name: "",
      },
      runtime,
    );
    if (chosen) await load(chosen);
  };

  const close = async () => {
    if (await confirmDiscard()) useWindowStore.getState().closeWindow(winId);
  };

  /* editing helpers (operate on the textarea selection) */

  const editor = () => editorRef.current;

  const replaceSelection = (text: string) => {
    const el = editor();
    if (!el || readonly) return;
    el.focus();
    // execCommand keeps the native undo stack working
    if (!document.execCommand("insertText", false, text)) {
      const { selectionStart: s, selectionEnd: e } = el;
      setContent(el.value.slice(0, s) + text + el.value.slice(e));
    }
  };

  const exec = (command: "undo" | "redo" | "cut" | "copy" | "delete") => {
    const el = editor();
    if (!el) return;
    el.focus();
    document.execCommand(command);
  };

  const paste = async () => {
    try {
      replaceSelection(await navigator.clipboard.readText());
    } catch {
      exec("delete");
    }
  };

  const selectAll = () => {
    const el = editor();
    el?.focus();
    el?.select();
  };

  const find = async () => {
    const el = editor();
    if (!el) return;
    const term = await showPrompt({ title: "Find", label: "Find what:", value: el.value.slice(el.selectionStart, el.selectionEnd), okLabel: "Find Next" });
    if (!term) return;
    const from = el.selectionEnd;
    let at = el.value.toLowerCase().indexOf(term.toLowerCase(), from);
    if (at < 0) at = el.value.toLowerCase().indexOf(term.toLowerCase());
    if (at < 0) {
      setStatus(`Cannot find "${term}"`);
      return;
    }
    el.focus();
    el.setSelectionRange(at, at + term.length);
  };

  const goTo = async () => {
    const el = editor();
    if (!el) return;
    const value = await showPrompt({ title: "Go To Line", label: "Line number:", value: String(caret.line), okLabel: "Go To" });
    const line = Number(value);
    if (!value || !Number.isFinite(line) || line < 1) return;
    const lines = el.value.split("\n");
    const offset = lines.slice(0, Math.min(line, lines.length) - 1).reduce((n, l) => n + l.length + 1, 0);
    el.focus();
    el.setSelectionRange(offset, offset);
  };

  const updateCaret = () => {
    const el = editor();
    if (!el) return;
    const before = el.value.slice(0, el.selectionStart).split("\n");
    setCaret({ line: before.length, col: (before.at(-1)?.length ?? 0) + 1 });
  };

  const insertDate = () => replaceSelection(new Date().toLocaleString());

  const menus: MenuBarItem[] = [
    {
      label: "File",
      items: [
        { label: "New", action: () => void newFile() },
        { label: "Open…", action: () => void openFile() },
        { label: "Save", disabled: readonly, action: () => void saveTo(path || null) },
        { label: "Save As…", disabled: readonly, action: () => void saveTo(null) },
        SEPARATOR,
        { label: "Exit", action: () => void close() },
      ],
    },
    {
      label: "Edit",
      items: [
        { label: "Undo", disabled: readonly, action: () => exec("undo") },
        { label: "Redo", disabled: readonly, action: () => exec("redo") },
        SEPARATOR,
        { label: "Cut", disabled: readonly, action: () => exec("cut") },
        { label: "Copy", action: () => exec("copy") },
        { label: "Paste", disabled: readonly, action: () => void paste() },
        { label: "Delete", disabled: readonly, action: () => exec("delete") },
        SEPARATOR,
        { label: "Find…", action: () => void find() },
        { label: "Go To…", action: () => void goTo() },
        SEPARATOR,
        { label: "Select All", action: selectAll },
        { label: "Time/Date", disabled: readonly, action: insertDate },
      ],
    },
    {
      label: "Format",
      items: [{ label: "Word Wrap", checked: wordWrap, action: () => setWordWrap((v) => !v) }],
    },
    {
      label: "View",
      items: [
        {
          label: "Zoom",
          children: [
            { label: "Zoom In", action: () => setZoom((z) => Math.min(500, z + 10)) },
            { label: "Zoom Out", action: () => setZoom((z) => Math.max(10, z - 10)) },
            { label: "Restore Default Zoom", action: () => setZoom(100) },
          ],
        },
        { label: "Status Bar", checked: showStatus, action: () => setShowStatus((v) => !v) },
      ],
    },
    {
      label: "Help",
      items: [
        {
          label: `About ${APP_NAME}`,
          action: () =>
            void showMessageBox({
              kind: "message_box",
              title: `About ${APP_NAME}`,
              text: `${APP_NAME}\nA simple UTF-8 text editor for the WebWINE virtual file system.`,
              style: 0x40,
            } as never),
        },
      ],
    },
  ];

  const onKeyDown = (e: React.KeyboardEvent) => {
    const mod = e.ctrlKey || e.metaKey;
    if (!mod) {
      if (e.key === "F5") {
        e.preventDefault();
        insertDate();
      }
      return;
    }
    const key = e.key.toLowerCase();
    const actions: Record<string, () => void> = {
      s: () => void saveTo(e.shiftKey ? null : path || null),
      o: () => void openFile(),
      n: () => void newFile(),
      f: () => void find(),
      g: () => void goTo(),
      "+": () => setZoom((z) => Math.min(500, z + 10)),
      "=": () => setZoom((z) => Math.min(500, z + 10)),
      "-": () => setZoom((z) => Math.max(10, z - 10)),
      "0": () => setZoom(100),
    };
    if (actions[key]) {
      e.preventDefault();
      actions[key]();
    }
  };

  const lineCount = useMemo(() => (content ? content.split("\n").length : 1), [content]);

  return (
    <div className="h-full min-h-0 flex flex-col" onKeyDown={onKeyDown}>
      <MenuBar items={menus} />

      {error && (
        <div className="px-3 py-2 bg-[var(--ww-danger-bg)] border-b border-[var(--ww-danger-border)] text-[var(--ww-danger)]">
          {error}
        </div>
      )}

      {readonly && !error && (
        <div className="px-3 py-2 bg-[var(--ww-warn-bg)] border-b border-[var(--ww-warn-border)] text-[var(--ww-warn)]">
          This file is not valid UTF-8 text. Showing a read-only hexdump.
        </div>
      )}

      <textarea
        ref={editorRef}
        className="ww-editor flex-1 min-h-0 w-full resize-none border-0 outline-none rounded-none p-1.5 font-[Consolas,'Lucida_Console',monospace]"
        style={{ fontSize: `${(13 * zoom) / 100}px`, lineHeight: 1.35 }}
        value={content}
        readOnly={readonly || loading}
        spellCheck={false}
        wrap={wordWrap ? "soft" : "off"}
        onChange={(e) => {
          setContent(e.target.value);
          if (!readonly) setStatus("Modified");
          updateCaret();
        }}
        onSelect={updateCaret}
        onKeyUp={updateCaret}
        onClick={updateCaret}
      />

      {showStatus && (
        <StatusBar className="justify-between">
          <span className="truncate">
            {status}
            {dirty && !readonly ? " · unsaved changes" : ""}
          </span>
          <span className="flex gap-5 flex-none">
            <span>
              Ln {caret.line}, Col {caret.col}
            </span>
            <span>{lineCount} lines</span>
            <span>{zoom}%</span>
            <span>UTF-8</span>
          </span>
        </StatusBar>
      )}
    </div>
  );
}

function toHexDump(bytes: Uint8Array): string {
  const lines: string[] = [];

  for (let i = 0; i < bytes.length; i += 16) {
    const chunk = bytes.slice(i, i + 16);

    const hex = Array.from(chunk)
      .map((b) => b.toString(16).padStart(2, "0"))
      .join(" ");

    const ascii = Array.from(chunk)
      .map((b) => (b >= 0x20 && b < 0x7f ? String.fromCharCode(b) : "."))
      .join("");

    lines.push(`${i.toString(16).padStart(8, "0")}  ${hex.padEnd(47)}  ${ascii}`);

    if (lines.length > 2048) {
      lines.push("… (truncated)");
      break;
    }
  }

  return lines.join("\n");
}
