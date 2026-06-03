import { useMemo, useState } from "react";
import { useWindowStore } from "../../state/windowStore";

import type { DirectoryEntry } from "../../core/wasm/worker";
import { formatSize } from "../../shared/lib/utils";

type PropertiesTab = "general" | "details";

export function openProperties(entry: DirectoryEntry) {
  let winId = "";

  const close = () => {
    if (winId) {
      useWindowStore.getState().closeWindow(winId);
    }
  };

  winId = useWindowStore.getState().openWindow({
    title: `${entry.name} Properties`,
    icon: iconForEntry(entry),
    variant: "dialog",
    width: 420,
    height: 480,
    content: <PropertiesApp entry={entry} onClose={close} />,
  });
}

function PropertiesApp({
  entry,
  onClose,
}: {
  entry: DirectoryEntry;
  onClose: () => void;
}) {
  const [tab, setTab] = useState<PropertiesTab>("general");

  return (
    <div className="h-full min-h-0 flex flex-col bg-[#202020] text-[#f2f2f2] font-[var(--system-font)] text-[12px]">
      <div className="px-3 pt-3 flex-1 min-h-0 overflow-hidden">
        <div className="h-[30px] flex items-end gap-0">
          <TabButton active={tab === "general"} onClick={() => setTab("general")}>
            General
          </TabButton>
          <TabButton active={tab === "details"} onClick={() => setTab("details")}>
            Details
          </TabButton>
        </div>

        <div className="border border-[#555] bg-[#202020] h-[calc(100%-30px)] overflow-auto px-4 pt-4 pb-3">
          {tab === "general" && <GeneralTab entry={entry} />}
          {tab === "details" && <DetailsTab entry={entry} />}
        </div>
      </div>

      <div className="h-[52px] flex justify-end items-center gap-2 px-3 bg-[#202020]">
        <DialogButton primary onClick={onClose}>
          OK
        </DialogButton>

        <DialogButton onClick={onClose}>
          Cancel
        </DialogButton>

        <DialogButton disabled>
          Apply
        </DialogButton>
      </div>
    </div>
  );
}

function GeneralTab({ entry }: { entry: DirectoryEntry }) {
  const info = useMemo(() => getEntryInfo(entry), [entry]);

  return (
    <div>
      <div className="flex items-center gap-4 pb-4">
        <img
          src={iconForEntry(entry)}
          alt=""
          className="w-8 h-8 object-contain flex-none"
          draggable={false}
        />

        <input
          className="h-[24px] flex-1 min-w-0 px-1 rounded-none border border-[#555] bg-[#111111] text-[#f2f2f2] text-[12px] outline-none focus:border-[#0078d7]"
          value={entry.name}
          readOnly
          spellCheck={false}
        />
      </div>

      <Separator />

      <PropertyGrid
        rows={[
          ["Type of file:", info.typeLabel],
          ["Opens with:", info.opensWith],
        ]}
      />

      <Separator />

      <PropertyGrid
        rows={[
          ["Location:", info.location],
          ["Size:", info.sizeLabel],
          ["Size on disk:", info.sizeOnDisk],
        ]}
      />

      <Separator />

      <PropertyGrid
        rows={[
          ["Created:", "Unknown"],
          ["Modified:", "Unknown"],
          ["Accessed:", "Unknown"],
        ]}
      />

      <Separator />

      <div className="grid grid-cols-[86px_1fr] gap-x-2 items-start">
        <div className="text-[#d6d6d6] pt-[3px]">Attributes:</div>

        <div className="flex flex-wrap gap-x-4 gap-y-2">
          <CheckBox label="Read-only" disabled />
          <CheckBox label="Hidden" disabled />
        </div>
      </div>
    </div>
  );
}

function DetailsTab({ entry }: { entry: DirectoryEntry }) {
  const info = useMemo(() => getEntryInfo(entry), [entry]);

  const rows = [
    ["Name", entry.name],
    ["Full path", entry.path],
    ["Folder path", info.location],
    ["Type", info.typeLabel],
    ["Extension", info.extension || "None"],
    ["Kind", entry.kind],
    ["Size", entry.kind === "file" ? formatSize(entry.size) : ""],
    ["Raw size", entry.kind === "file" ? `${entry.size} bytes` : ""],
  ].filter(([, value]) => value !== "");

  return (
    <div>
      <div className="border border-[#3a3a3a]">
        <div className="grid grid-cols-[130px_1fr] h-[26px] bg-[#191919] border-b border-[#3a3a3a] text-[#a6a6a6]">
          <div className="px-2 flex items-center border-r border-[#3a3a3a]">
            Property
          </div>
          <div className="px-2 flex items-center">
            Value
          </div>
        </div>

        {rows.map(([label, value]) => (
          <div
            key={label}
            className="grid grid-cols-[130px_1fr] min-h-[26px] border-b border-[#2b2b2b] last:border-b-0 hover:bg-[rgba(255,255,255,0.06)]"
          >
            <div className="px-2 py-[5px] text-[#d6d6d6] border-r border-[#2b2b2b]">
              {label}
            </div>

            <div className="px-2 py-[5px] text-[#f2f2f2] break-all">
              {value}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function TabButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      className={[
        "h-[30px] px-4 rounded-none border text-[12px] cursor-default",
        "border-b-0",
        active
          ? "bg-[#202020] border-[#555] text-[#ffffff] relative z-10"
          : "bg-[#191919] border-[#3a3a3a] text-[#d6d6d6] hover:bg-[#242424]",
      ].join(" ")}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

function DialogButton({
  children,
  onClick,
  primary,
  disabled,
}: {
  children: React.ReactNode;
  onClick?: () => void;
  primary?: boolean;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      className={[
        "min-w-[76px] h-[26px] px-3",
        "rounded-none border text-[12px] font-normal",
        "cursor-default outline-none",
        "bg-[#333333] text-[#f2f2f2] border-[#555555]",
        "hover:bg-[#3f3f3f] hover:border-[#6b6b6b]",
        "active:bg-[#2b2b2b]",
        "focus:border-[#0078d7] focus:shadow-[inset_0_0_0_1px_#0078d7]",
        "disabled:opacity-50 disabled:pointer-events-none",
        primary ? "border-[#0078d7] shadow-[inset_0_0_0_1px_#0078d7]" : "",
      ].join(" ")}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

function PropertyGrid({ rows }: { rows: Array<[string, string]> }) {
  return (
    <div className="grid grid-cols-[86px_1fr] gap-x-2 gap-y-[7px]">
      {rows.map(([label, value]) => (
        <PropertyRow key={label} label={label} value={value} />
      ))}
    </div>
  );
}

function PropertyRow({ label, value }: { label: string; value: string }) {
  return (
    <>
      <div className="text-[#d6d6d6]">{label}</div>
      <div className="text-[#f2f2f2] break-all">{value}</div>
    </>
  );
}

function Separator() {
  return <div className="h-px bg-[#3a3a3a] my-3" />;
}

function CheckBox({
  label,
  disabled,
}: {
  label: string;
  disabled?: boolean;
}) {
  return (
    <label className="inline-flex items-center gap-2 text-[#f2f2f2]">
      <input
        type="checkbox"
        disabled={disabled}
        className="w-[13px] h-[13px] rounded-none accent-[#0078d7] disabled:opacity-60"
      />
      <span>{label}</span>
    </label>
  );
}

function getEntryInfo(entry: DirectoryEntry) {
  const extension = extensionOf(entry.name);
  const location = entry.path.split("\\").slice(0, -1).join("\\") || entry.path;

  const typeLabel =
    entry.kind === "directory"
      ? "File folder"
      : extension
        ? `${extension.toUpperCase()} File`
        : "File";

  const opensWith =
    entry.kind === "directory"
      ? "File Explorer"
      : extension === "exe"
        ? "Application"
        : extension === "lnk"
          ? "Windows Shortcut"
          : extension === "txt"
            ? "Notepad"
            : "Unknown application";

  const sizeLabel =
    entry.kind === "file"
      ? `${formatSize(entry.size)} (${entry.size.toLocaleString()} bytes)`
      : "Unknown";

  const sizeOnDisk =
    entry.kind === "file"
      ? `${formatSize(roundUpCluster(entry.size))} (${roundUpCluster(entry.size).toLocaleString()} bytes)`
      : "Unknown";

  return {
    extension,
    location,
    typeLabel,
    opensWith,
    sizeLabel,
    sizeOnDisk,
  };
}

function extensionOf(name: string): string | null {
  const base = name.split("\\").pop() || name;
  const dot = base.lastIndexOf(".");

  if (dot <= 0 || dot === base.length - 1) return null;

  return base.slice(dot + 1).toLowerCase();
}

function roundUpCluster(size: number): number {
  const cluster = 4096;

  if (size <= 0) return 0;

  return Math.ceil(size / cluster) * cluster;
}

function iconForEntry(entry: DirectoryEntry): string {
  if (entry.kind === "directory") {
    return `/theme/icons/shell/folder.webp`;
  }

  const ext = extensionOf(entry.name);

  if (ext === "exe") return `/theme/icons/shell/default_executable.webp`;
  if (ext === "lnk") return `/theme/icons/shell/shortcut.webp`;
  if (ext === "txt" || ext === "log" || ext === "ini") {
    return `/theme/icons/shell/text.webp`;
  }

  return `/theme/icons/shell/file.webp`;
}