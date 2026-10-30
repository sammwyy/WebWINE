import { useMemo } from "react";
import { useWindowStore } from "@/state/windowStore";

import type { DirectoryEntry } from "@/core/wasm/worker";
import type { RuntimeBridge } from "@/core/bridge/runtime-bridge";
import { formatSize } from "@/shared/lib/utils";
import { resolveIcon } from "@/shared/lib/icons/icon-resolver";
import { AppRegistry } from "@/shared/lib/app-registry";
import { Button, Checkbox, DialogFooter, Input, ListView, Tabs } from "@/shared/ui";

export async function openProperties(entry: DirectoryEntry, runtime: RuntimeBridge) {
  let winId = "";

  const icon = await resolveIcon(entry, runtime);

  const close = () => {
    if (winId) {
      useWindowStore.getState().closeWindow(winId);
    }
  };

  winId = useWindowStore.getState().openWindow({
    title: `${entry.name} Properties`,
    icon: icon.src,
    variant: "dialog",
    width: 400,
    height: 480,
    content: <PropertiesApp entry={entry} iconSrc={icon.src} onClose={close} />,
  });
}

function PropertiesApp({
  entry,
  iconSrc,
  onClose,
}: {
  entry: DirectoryEntry;
  iconSrc: string;
  onClose: () => void;
}) {
  return (
    <div className="h-full min-h-0 flex flex-col">
      <Tabs
        className="flex-1 min-h-0 px-2 pt-2"
        tabs={[
          { id: "general", label: "General", content: <GeneralTab entry={entry} iconSrc={iconSrc} /> },
          { id: "details", label: "Details", content: <DetailsTab entry={entry} /> },
        ]}
      />

      <DialogFooter>
        <Button isDefault onClick={onClose}>
          OK
        </Button>
        <Button onClick={onClose}>Cancel</Button>
        <Button disabled>Apply</Button>
      </DialogFooter>
    </div>
  );
}

function GeneralTab({ entry, iconSrc }: { entry: DirectoryEntry; iconSrc: string }) {
  const info = useMemo(() => getEntryInfo(entry), [entry]);

  return (
    <div className="flex flex-col">
      <div className="flex items-center gap-4 pb-3">
        <img src={iconSrc} alt="" className="w-8 h-8 object-contain flex-none" draggable={false} />
        <Input className="flex-1 min-w-0" value={entry.name} readOnly spellCheck={false} />
      </div>

      <div className="ww-separator my-3" />

      <PropertyGrid
        rows={[
          ["Type of file:", info.typeLabel],
          ["Opens with:", info.opensWith],
        ]}
      />

      <div className="ww-separator my-3" />

      <PropertyGrid
        rows={[
          ["Location:", info.location],
          ["Size:", info.sizeLabel],
          ["Size on disk:", info.sizeOnDisk],
        ]}
      />

      <div className="ww-separator my-3" />

      <PropertyGrid
        rows={[
          ["Created:", "Unknown"],
          ["Modified:", "Unknown"],
          ["Accessed:", "Unknown"],
        ]}
      />

      <div className="ww-separator my-3" />

      <div className="ww-propgrid items-center">
        <span>Attributes:</span>
        <div className="flex flex-wrap gap-x-4 gap-y-2">
          <Checkbox label="Read-only" disabled />
          <Checkbox label="Hidden" disabled />
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
    <div className="flex h-full min-h-[260px] -m-[10px]">
      <ListView
        items={rows}
        getKey={([label]) => label}
        columns={[
          { key: "prop", label: "Property", width: 120, render: ([label]) => label },
          { key: "value", label: "Value", width: 230, render: ([, value]) => <span title={value}>{value}</span> },
        ]}
      />
    </div>
  );
}

function PropertyGrid({ rows }: { rows: Array<[string, string]> }) {
  return (
    <div className="ww-propgrid">
      {rows.map(([label, value]) => (
        <div key={label} className="contents">
          <span>{label}</span>
          <span>{value}</span>
        </div>
      ))}
    </div>
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

  let opensWith = "Unknown application";
  if (entry.kind === "directory" || entry.kind === "link") {
    opensWith = AppRegistry.getAppByAction("explorer")?.name || "WW Explorer";
  } else if (extension === "exe" || extension === "dll") {
    opensWith = "Application";
  } else if (extension === "lnk") {
    opensWith = "Windows Shortcut";
  } else if (extension) {
    const apps = AppRegistry.getAppsForExtension(extension);
    if (apps.length > 0) {
      opensWith = apps[0].name;
    } else {
      opensWith = AppRegistry.getAppByAction("editor")?.name || "WW Editor";
    }
  }

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
