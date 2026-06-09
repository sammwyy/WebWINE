/**
 * RegeditApp — a minimal registry editor over the in-memory hive exposed by the
 * WASM runtime. Mirrors the structure of the other virtual apps: an `openRegedit`
 * entry point creates a window, the component talks to the registry through the
 * RuntimeBridge (regListSubkeys / regListValues / regSetValue / ...), and listens
 * to `onRegistryChanged` so edits from a guest or another window refresh the view.
 */

import { useCallback, useEffect, useMemo, useState } from "react";

import { useWindowStore } from "@/state/windowStore";
import type { NamedValue, RegValue, RuntimeBridge } from "@/core/bridge/runtime-bridge";

const ROOTS = [
  "HKEY_CLASSES_ROOT",
  "HKEY_CURRENT_USER",
  "HKEY_LOCAL_MACHINE",
  "HKEY_USERS",
  "HKEY_CURRENT_CONFIG",
];

export function openRegedit(runtime: RuntimeBridge) {
  const id = useWindowStore.getState().openWindow({
    title: "WebWINE: Registry Editor",
    icon: `${import.meta.env.BASE_URL}theme/icons/apps/regedit.webp`,
    width: 880,
    height: 560,
    content: <div />,
  });
  useWindowStore.getState().setContent(id, <RegeditApp runtime={runtime} />);
}

function typeName(v: RegValue): string {
  switch (v.type) {
    case "Sz": return "REG_SZ";
    case "ExpandSz": return "REG_EXPAND_SZ";
    case "Dword": return "REG_DWORD";
    case "Qword": return "REG_QWORD";
    case "Binary": return "REG_BINARY";
    case "MultiSz": return "REG_MULTI_SZ";
    case "None": return "REG_NONE";
  }
}

function formatData(v: RegValue): string {
  switch (v.type) {
    case "Sz":
    case "ExpandSz": return v.data;
    case "Dword": return `0x${(v.data >>> 0).toString(16).padStart(8, "0")} (${v.data >>> 0})`;
    case "Qword": return String(v.data);
    case "Binary": return v.data.map((b) => b.toString(16).padStart(2, "0")).join(" ");
    case "MultiSz": return v.data.join(", ");
    case "None": return "(zero-length binary value)";
  }
}

/** Prompt the user to edit `current` and return a new RegValue, or null to cancel. */
function editValue(current: RegValue): RegValue | null {
  switch (current.type) {
    case "Sz":
    case "ExpandSz": {
      const s = window.prompt("Value data:", current.data);
      return s === null ? null : { type: current.type, data: s };
    }
    case "Dword":
    case "Qword": {
      const s = window.prompt("Value data (decimal or 0x hex):", String(current.data));
      if (s === null) return null;
      const n = s.trim().toLowerCase().startsWith("0x") ? parseInt(s.trim(), 16) : parseInt(s.trim(), 10);
      if (Number.isNaN(n)) return null;
      return { type: current.type, data: n };
    }
    case "MultiSz": {
      const s = window.prompt("Values (comma-separated):", current.data.join(","));
      return s === null ? null : { type: "MultiSz", data: s.split(",").map((x) => x.trim()).filter(Boolean) };
    }
    case "Binary": {
      const s = window.prompt("Bytes (hex, space-separated):", current.data.map((b) => b.toString(16).padStart(2, "0")).join(" "));
      if (s === null) return null;
      const bytes = s.split(/\s+/).filter(Boolean).map((h) => parseInt(h, 16) & 0xff);
      return { type: "Binary", data: bytes };
    }
    case "None":
      return null;
  }
}

function TreeNode({
  path,
  name,
  depth,
  selected,
  onSelect,
  runtime,
  refreshToken,
}: {
  path: string;
  name: string;
  depth: number;
  selected: string;
  onSelect: (path: string) => void;
  runtime: RuntimeBridge;
  refreshToken: number;
}) {
  const [expanded, setExpanded] = useState(depth === 0 && name.startsWith("HKEY_LOCAL") ? false : false);
  const [children, setChildren] = useState<string[] | null>(null);

  const load = useCallback(async () => {
    setChildren(await runtime.regListSubkeys(path));
  }, [path, runtime]);

  useEffect(() => {
    if (expanded) void load();
  }, [expanded, load, refreshToken]);

  const isSel = selected === path;
  return (
    <div>
      <div
        className={`flex items-center gap-1 px-1 py-[2px] cursor-pointer text-[13px] whitespace-nowrap ${isSel ? "bg-[var(--accent,#2563eb)] text-white" : "hover:bg-[rgba(127,127,127,0.15)]"}`}
        style={{ paddingLeft: depth * 14 + 4 }}
        onClick={() => onSelect(path)}
      >
        <span
          className="w-3 inline-flex justify-center text-[10px] opacity-70"
          onClick={(e) => {
            e.stopPropagation();
            setExpanded((v) => !v);
          }}
        >
          {expanded ? "▾" : "▸"}
        </span>
        <span>{name}</span>
      </div>
      {expanded &&
        (children ?? []).map((c) => (
          <TreeNode
            key={c}
            path={`${path}\\${c}`}
            name={c}
            depth={depth + 1}
            selected={selected}
            onSelect={onSelect}
            runtime={runtime}
            refreshToken={refreshToken}
          />
        ))}
    </div>
  );
}

function RegeditApp({ runtime }: { runtime: RuntimeBridge }) {
  const [selected, setSelected] = useState<string>("HKEY_CURRENT_USER");
  const [values, setValues] = useState<NamedValue[]>([]);
  const [refreshToken, setRefreshToken] = useState(0);

  const refresh = useCallback(() => setRefreshToken((t) => t + 1), []);

  useEffect(() => {
    const off = runtime.onRegistryChanged(refresh);
    return off;
  }, [runtime, refresh]);

  useEffect(() => {
    let alive = true;
    void runtime.regListValues(selected).then((v) => {
      if (alive) setValues(v);
    });
    return () => {
      alive = false;
    };
  }, [selected, runtime, refreshToken]);

  const rows = useMemo(() => {
    // Show the key's default value first (named "" -> "(Default)").
    const sorted = [...values].sort((a, b) => (a.name === "" ? -1 : b.name === "" ? 1 : a.name.localeCompare(b.name)));
    if (!sorted.some((r) => r.name === "")) {
      sorted.unshift({ name: "", value: { type: "Sz", data: "" } });
    }
    return sorted;
  }, [values]);

  const onEdit = async (row: NamedValue) => {
    const next = editValue(row.value);
    if (next) {
      await runtime.regSetValue(selected, row.name, next);
    }
  };

  const onNewValue = async (kind: "Sz" | "Dword") => {
    const name = window.prompt("New value name:");
    if (!name) return;
    const init: RegValue = kind === "Sz" ? { type: "Sz", data: "" } : { type: "Dword", data: 0 };
    await runtime.regSetValue(selected, name, init);
  };

  const onNewKey = async () => {
    const name = window.prompt("New key name:");
    if (!name) return;
    await runtime.regCreateKey(`${selected}\\${name}`);
  };

  const onDeleteKey = async () => {
    if (ROOTS.includes(selected)) return;
    if (!window.confirm(`Delete key "${selected}" and all its subkeys?`)) return;
    await runtime.regDeleteKey(selected);
    const parent = selected.split("\\").slice(0, -1).join("\\");
    setSelected(parent || "HKEY_CURRENT_USER");
  };

  const onDeleteValue = async (row: NamedValue) => {
    if (row.name === "") return;
    if (!window.confirm(`Delete value "${row.name}"?`)) return;
    await runtime.regDeleteValue(selected, row.name);
  };

  return (
    <div className="flex flex-col h-full bg-[var(--window-bg,#1e1e1e)] text-[var(--text,#ddd)]">
      {/* Toolbar */}
      <div className="flex items-center gap-2 px-2 py-1 border-b border-[rgba(127,127,127,0.25)] text-[12px]">
        <button className="px-2 py-[2px] rounded hover:bg-[rgba(127,127,127,0.2)]" onClick={onNewKey}>New Key</button>
        <button className="px-2 py-[2px] rounded hover:bg-[rgba(127,127,127,0.2)]" onClick={() => onNewValue("Sz")}>New String</button>
        <button className="px-2 py-[2px] rounded hover:bg-[rgba(127,127,127,0.2)]" onClick={() => onNewValue("Dword")}>New DWORD</button>
        <button className="px-2 py-[2px] rounded hover:bg-[rgba(127,127,127,0.2)]" onClick={onDeleteKey}>Delete Key</button>
      </div>

      <div className="flex flex-1 min-h-0">
        {/* Key tree */}
        <div className="w-1/3 min-w-[200px] overflow-auto border-r border-[rgba(127,127,127,0.25)] py-1">
          {ROOTS.map((r) => (
            <TreeNode
              key={r}
              path={r}
              name={r}
              depth={0}
              selected={selected}
              onSelect={setSelected}
              runtime={runtime}
              refreshToken={refreshToken}
            />
          ))}
        </div>

        {/* Value list */}
        <div className="flex-1 overflow-auto">
          <table className="w-full border-collapse text-[13px]">
            <thead>
              <tr className="text-left sticky top-0 bg-[var(--window-bg,#1e1e1e)]">
                <th className="font-normal px-2 py-1 border-b border-[rgba(127,127,127,0.25)]">Name</th>
                <th className="font-normal px-2 py-1 border-b border-[rgba(127,127,127,0.25)]">Type</th>
                <th className="font-normal px-2 py-1 border-b border-[rgba(127,127,127,0.25)]">Data</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <tr
                  key={row.name || "(default)"}
                  className="hover:bg-[rgba(127,127,127,0.12)] cursor-default"
                  onDoubleClick={() => onEdit(row)}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    void onDeleteValue(row);
                  }}
                >
                  <td className="px-2 py-[3px] whitespace-nowrap">{row.name === "" ? "(Default)" : row.name}</td>
                  <td className="px-2 py-[3px] whitespace-nowrap opacity-80">{typeName(row.value)}</td>
                  <td className="px-2 py-[3px] opacity-90">{formatData(row.value)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Status bar: current path */}
      <div className="px-2 py-1 border-t border-[rgba(127,127,127,0.25)] text-[12px] opacity-70 whitespace-nowrap overflow-hidden text-ellipsis">
        {selected}
      </div>
    </div>
  );
}
