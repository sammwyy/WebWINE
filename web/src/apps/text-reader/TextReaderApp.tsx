import { useEffect, useMemo, useState } from "react";
import { useWindowStore } from "@/state/windowStore";
import type { RuntimeBridge } from "@/core/bridge/runtime-bridge";
import { basename } from "@/shared/lib/utils";
import { resolveIcon } from "@/shared/lib/icons/icon-resolver";

export async function openTextReader(path: string, runtime: RuntimeBridge) {
  const name = path ? basename(path) : "Untitled";

  const resolved = await resolveIcon(
    { name, path, kind: "file", size: 0 },
    runtime,
  );

  const icon = resolved?.src || `${import.meta.env.BASE_URL}theme/icons/exts/txt.webp`;

  let winId = "";

  winId = useWindowStore.getState().openWindow({
    title: `${name} — WebWINE: Text Editor`,
    icon,
    width: 720,
    height: 520,
    content: (
      <TextEditorApp
        path={path}
        runtime={runtime}
        winId={() => winId}
      />
    ),
  });
}

function TextEditorApp({
  path,
  runtime,
  winId,
}: {
  path: string;
  runtime: RuntimeBridge;
  winId?: () => string;
}) {
  const name = useMemo(() => path ? basename(path) : "Untitled", [path]);

  const [content, setContent] = useState("");
  const [originalContent, setOriginalContent] = useState("");
  const [readonly, setReadonly] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  const [status, setStatus] = useState("Loading…");
  const [error, setError] = useState<string | null>(null);

  const dirty = content !== originalContent;

  useEffect(() => {
    let alive = true;

    setLoading(true);
    setReadonly(false);
    setError(null);
    setStatus("Loading…");
    setContent("");
    setOriginalContent("");

    if (!path) {
      setLoading(false);
      setReadonly(false);
      setStatus("New file");
      return;
    }

    runtime
      .readFile(path)
      .then((bytes) => {
        if (!alive) return;

        const decoder = new TextDecoder("utf-8", { fatal: true });

        try {
          const text = decoder.decode(bytes);

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
      })
      .catch((err) => {
        if (!alive) return;

        const message = `Error: ${err}`;

        setError(message);
        setContent(message);
        setOriginalContent(message);
        setReadonly(true);
        setStatus("Failed to load");
      })
      .finally(() => {
        if (!alive) return;
        setLoading(false);
      });

    return () => {
      alive = false;
    };
  }, [path, runtime]);

  useEffect(() => {
    if (!winId) return;

    const id = winId();
    if (!id) return;

    useWindowStore
      .getState()
      .setTitle(id, `${dirty ? "*" : ""}${name} — WebWINE: Text Editor`);
  }, [dirty, name, winId]);

  const save = async () => {
    if (readonly || loading || saving) return;

    setSaving(true);
    setError(null);
    setStatus("Saving…");

    try {
      const savePath = path || "C:\\Users\\guest\\Documents\\Untitled.txt";
      const encoder = new TextEncoder();
      const bytes = encoder.encode(content);

      await runtime.mountFile(savePath, bytes.buffer);

      setOriginalContent(content);
      setStatus(`${bytes.length.toLocaleString()} bytes · saved`);
    } catch (err) {
      const message = `Save failed: ${err}`;

      setError(message);
      setStatus("Save failed");
    } finally {
      setSaving(false);
    }
  };

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        void save();
      }
    };

    document.addEventListener("keydown", onKeyDown);

    return () => {
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [save]);

  const lineCount = useMemo(() => {
    if (!content) return 1;
    return content.split("\n").length;
  }, [content]);

  const columnCount = useMemo(() => {
    const lastLine = content.split("\n").at(-1) ?? "";
    return lastLine.length + 1;
  }, [content]);

  return (
    <div className="h-full min-h-0 flex flex-col bg-[#111111] text-[#f2f2f2] font-[var(--system-font)]">
      <div className="h-[34px] flex items-center gap-1 px-1 bg-[#202020] border-b border-[#2b2b2b] text-[12px]">
        <MenuButton disabled>File</MenuButton>
        <MenuButton disabled>Edit</MenuButton>
        <MenuButton disabled>Format</MenuButton>
        <MenuButton disabled>View</MenuButton>
        <MenuButton disabled>Help</MenuButton>

        <div className="flex-1" />

        <button
          type="button"
          disabled={readonly || loading || saving || !dirty}
          className={[
            "h-[26px] min-w-[72px] px-3 rounded-none border text-[12px]",
            "bg-[#333333] text-[#f2f2f2] border-[#555555]",
            "hover:bg-[#3f3f3f] hover:border-[#6b6b6b]",
            "active:bg-[#2b2b2b]",
            "focus:outline-none focus:border-[#0078d7]",
            "disabled:opacity-45 disabled:pointer-events-none",
          ].join(" ")}
          onClick={() => void save()}
        >
          {saving ? "Saving…" : "Save"}
        </button>
      </div>

      {error && (
        <div className="px-3 py-2 bg-[#241414] border-b border-[#5c2b2b] text-[#ffb3bd] text-[12px]">
          {error}
        </div>
      )}

      {readonly && !error && (
        <div className="px-3 py-2 bg-[#201a0d] border-b border-[#5c4a1f] text-[#f9e2af] text-[12px]">
          This file is not valid UTF-8 text. Showing a read-only hexdump.
        </div>
      )}

      <textarea
        className={[
          "flex-1 min-h-0 w-full resize-none",
          "bg-[#111111] text-[#f2f2f2]",
          "border-0 outline-none rounded-none",
          "p-3",
          "text-[13px] leading-[18px]",
          "font-[Cascadia_Code,Consolas,monospace]",
          "selection:bg-[#0078d7] selection:text-white",
          readonly ? "cursor-default text-[#d6d6d6]" : "",
        ].join(" ")}
        value={content}
        readOnly={readonly || loading}
        spellCheck={false}
        wrap="off"
        onChange={(e) => {
          setContent(e.target.value);

          if (!readonly) {
            setStatus("Modified");
          }
        }}
      />

      <div className="h-[24px] flex items-center justify-between gap-3 px-3 bg-[#202020] border-t border-[#2b2b2b] text-[11px] text-[#a6a6a6]">
        <span className="truncate">
          {status}
          {dirty && !readonly ? " · unsaved changes" : ""}
        </span>

        <span className="flex-none">
          Ln {lineCount}, Col {columnCount}
        </span>

        <span className="flex-none">
          UTF-8
        </span>
      </div>
    </div>
  );
}

function MenuButton({
  children,
  disabled,
}: {
  children: React.ReactNode;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      className={[
        "h-[28px] px-3 rounded-none border border-transparent",
        "bg-transparent text-[#f2f2f2] text-[12px]",
        "cursor-default",
        "hover:bg-[rgba(255,255,255,0.10)]",
        "disabled:opacity-100 disabled:pointer-events-none",
      ].join(" ")}
    >
      {children}
    </button>
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

    lines.push(
      `${i.toString(16).padStart(8, "0")}  ${hex.padEnd(47)}  ${ascii}`,
    );

    if (lines.length > 2048) {
      lines.push("… (truncated)");
      break;
    }
  }

  return lines.join("\n");
}