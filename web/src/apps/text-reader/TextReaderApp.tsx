import { useEffect, useState } from "react";
import { useWindowStore } from "../../stores/useWindowStore.js";
import type { RuntimeBridge } from "../../lib/runtime-bridge.js";
import { basename } from "../../lib/utils.js";

import { useThemeStore } from "../../stores/useThemeStore.js";
import { resolveIcon } from "../../lib/icon-resolver.js";

export async function openTextReader(path: string, runtime: RuntimeBridge) {
  const theme = useThemeStore.getState().theme;
  const name = basename(path);

  const resolved = await resolveIcon(
    { name, path, kind: "file", size: 0 },
    runtime
  );
  const icon = resolved?.src || `/themes/${theme}/icons/exts/txt.webp`;

  useWindowStore.getState().openWindow({
    title: `${name} — Raw Viewer`,
    icon,
    width: 640,
    height: 460,
    content: <TextReaderApp path={path} runtime={runtime} />,
  });
}

function TextReaderApp({
  path,
  runtime,
}: {
  path: string;
  runtime: RuntimeBridge;
}) {
  const [content, setContent] = useState<string>("Loading…");

  useEffect(() => {
    runtime
      .readFile(path)
      .then((bytes) => {
        const decoder = new TextDecoder("utf-8", { fatal: true });
        try {
          setContent(decoder.decode(bytes));
        } catch {
          setContent(toHexDump(bytes));
        }
      })
      .catch((err) => {
        setContent(`Error: ${err}`);
      });
  }, [path, runtime]);

  return <pre className="raw-viewer-content">{content}</pre>;
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
