import React, { useEffect, useMemo, useState } from "react";

import { useWindowStore } from "../../state/windowStore";
import type { RuntimeBridge } from "../../core/bridge/runtime-bridge";
import type { PeInfo, PeSection, PeImportModule } from "../../core/wasm/worker";
import { basename, formatSize } from "../../shared/lib/utils";
import { log } from "../../state/logStore";
import { resolveIcon } from "../../shared/lib/icon-resolver";

export async function openPeInspector(path: string, runtime: RuntimeBridge) {
  const name = basename(path);

  const resolved = await resolveIcon(
    { name, path, kind: "file", size: 0 },
    runtime,
  );

  const icon = resolved?.src || `/theme/icons/shell/default_executable.webp`;

  let winId = "";

  winId = useWindowStore.getState().openWindow({
    title: `Properties: ${name}`,
    icon,
    width: 760,
    height: 560,
    content: (
      <PeInspectorApp
        path={path}
        name={name}
        runtime={runtime}
        winId={() => winId}
        icon={icon}
      />
    ),
  });
}

type TabId = "overview" | "sections" | "imports";

function PeInspectorApp({
  path,
  name,
  runtime,
  winId,
  icon,
}: {
  path: string;
  name: string;
  runtime: RuntimeBridge;
  winId?: () => string;
  icon: string;
}) {
  const [info, setInfo] = useState<PeInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<TabId>("overview");
  const [reloadKey, setReloadKey] = useState(0);

  useEffect(() => {
    let alive = true;

    setInfo(null);
    setError(null);

    runtime
      .inspectPe(path)
      .then((res) => {
        if (!alive) return;

        setInfo(res);

        log(
          "pe",
          `${name}: ${res.machine} ${res.subsystem} image_base=${hex(res.image_base, 8)} entry=${hex(res.entry_point_rva, 8)}`,
        );

        if (winId) {
          const id = winId();

          useWindowStore
            .getState()
            .setTitle(
              id,
              `${name} — ${res.is_dll ? "DLL" : "EXE"} · ${res.machine} · ${res.subsystem}`,
            );
        }
      })
      .catch((err) => {
        if (!alive) return;

        setError(String(err));
        log("pe", `failed to parse ${name}: ${err}`, "error");
      });

    return () => {
      alive = false;
    };
  }, [path, name, runtime, winId, reloadKey]);

  if (error) {
    return (
      <div className="h-full flex flex-col bg-[#111111] text-[#f2f2f2] font-[var(--system-font)]">
        <InspectorToolbar
          name={name}
          path={path}
          icon={icon}
          onReload={() => setReloadKey((v) => v + 1)}
        />

        <div className="m-4 border border-[#5c2b2b] bg-[#241414] text-[#ffb3bd] p-4 text-[12px] leading-[18px]">
          <div className="font-semibold mb-1">Failed to parse PE</div>
          <div className="font-[Cascadia_Code,Consolas,monospace] whitespace-pre-wrap break-words">
            {error}
          </div>
        </div>
      </div>
    );
  }

  if (!info) {
    return (
      <div className="h-full flex flex-col bg-[#111111] text-[#f2f2f2] font-[var(--system-font)]">
        <InspectorToolbar
          name={name}
          path={path}
          icon={icon}
          onReload={() => setReloadKey((v) => v + 1)}
        />

        <div className="flex-1 grid place-items-center text-[12px] text-[#a6a6a6]">
          Parsing PE headers…
        </div>
      </div>
    );
  }

  const warnings = getWarnings(info);

  return (
    <div className="h-full min-h-0 flex flex-col bg-[#111111] text-[#f2f2f2] font-[var(--system-font)]">
      <InspectorToolbar
        name={name}
        path={path}
        icon={icon}
        onReload={() => setReloadKey((v) => v + 1)}
      />

      <SummaryStrip info={info} warnings={warnings} />

      <div className="h-[34px] flex items-end gap-0 px-2 bg-[#191919] border-b border-[#2b2b2b]">
        <TabButton active={tab === "overview"} onClick={() => setTab("overview")}>
          Overview
        </TabButton>
        <TabButton active={tab === "sections"} onClick={() => setTab("sections")}>
          Sections
        </TabButton>
        <TabButton active={tab === "imports"} onClick={() => setTab("imports")}>
          Imports
        </TabButton>
      </div>

      <div className="flex-1 min-h-0 overflow-auto">
        {tab === "overview" && <Overview info={info} path={path} />}
        {tab === "sections" && <Sections sections={info.sections} />}
        {tab === "imports" && <Imports imports={info.imports} />}
      </div>
    </div>
  );
}

function InspectorToolbar({
  name,
  path,
  icon,
  onReload,
}: {
  name: string;
  path: string;
  icon: string;
  onReload: () => void;
}) {
  return (
    <div className="h-[58px] flex items-center gap-3 px-3 bg-[#202020] border-b border-[#2b2b2b]">
      <img
        src={icon}
        alt=""
        className="w-8 h-8 object-contain flex-none"
        draggable={false}
      />

      <div className="min-w-0 flex-1">
        <div className="text-[13px] text-[#f2f2f2] truncate">{name}</div>
        <div className="mt-[2px] text-[11px] text-[#a6a6a6] font-[Cascadia_Code,Consolas,monospace] truncate">
          {path}
        </div>
      </div>

      <button
        type="button"
        className="h-[28px] min-w-[78px] px-3 rounded-none border border-[#555] bg-[#333] text-[#f2f2f2] text-[12px] cursor-default hover:bg-[#3f3f3f] hover:border-[#6b6b6b] active:bg-[#2b2b2b] focus:outline-none focus:border-[#0078d7]"
        onClick={() => {
          void navigator.clipboard?.writeText(path).catch(() => { });
        }}
      >
        Copy path
      </button>

      <button
        type="button"
        className="h-[28px] min-w-[70px] px-3 rounded-none border border-[#555] bg-[#333] text-[#f2f2f2] text-[12px] cursor-default hover:bg-[#3f3f3f] hover:border-[#6b6b6b] active:bg-[#2b2b2b] focus:outline-none focus:border-[#0078d7]"
        onClick={onReload}
      >
        Refresh
      </button>
    </div>
  );
}

function SummaryStrip({
  info,
  warnings,
}: {
  info: PeInfo;
  warnings: string[];
}) {
  return (
    <div className="grid grid-cols-[repeat(4,minmax(0,1fr))] max-[680px]:grid-cols-2 border-b border-[#2b2b2b] bg-[#151515]">
      <SummaryItem label="Type" value={info.is_dll ? "DLL" : info.is_pe32 ? "PE32" : "PE32+"} />
      <SummaryItem label="Machine" value={info.machine} />
      <SummaryItem label="Subsystem" value={info.subsystem} />
      <SummaryItem
        label="Warnings"
        value={warnings.length ? `${warnings.length}` : "None"}
        danger={warnings.length > 0}
      />
    </div>
  );
}

function SummaryItem({
  label,
  value,
  danger,
}: {
  label: string;
  value: string;
  danger?: boolean;
}) {
  return (
    <div className="min-w-0 px-3 py-2 border-r border-[#2b2b2b] last:border-r-0">
      <div className="text-[10px] uppercase tracking-[0.05em] text-[#a6a6a6]">
        {label}
      </div>
      <div
        className={[
          "mt-[2px] text-[12px] font-[Cascadia_Code,Consolas,monospace] truncate",
          danger ? "text-[#ffb3bd]" : "text-[#f2f2f2]",
        ].join(" ")}
      >
        {value}
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
        "h-[30px] px-4 rounded-none border border-transparent border-b-0",
        "text-[12px] cursor-default",
        active
          ? "bg-[#111111] border-[#2b2b2b] text-[#ffffff]"
          : "bg-transparent text-[#d6d6d6] hover:bg-[rgba(255,255,255,0.08)]",
      ].join(" ")}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

function Overview({ info, path }: { info: PeInfo; path: string }) {
  const fields = [
    ["Path", path],
    ["Machine", `${info.machine} (${hex(info.machine_id, 4)})`],
    ["Type", info.is_dll ? "DLL" : info.is_pe32 ? "PE32 Executable" : "PE32+ Executable"],
    ["Subsystem", `${info.subsystem} (${info.subsystem_id})`],
    ["Image Base", hex(info.image_base, info.is_pe32 ? 8 : 16)],
    ["Entry Point", `${hex(info.entry_point_rva, 8)} (RVA)`],
    ["Image Size", `${formatSize(info.size_of_image)} (${hex(info.size_of_image, 8)})`],
    ["Sections", String(info.sections.length)],
    [
      "Imports",
      `${info.imports.length} DLL${info.imports.length !== 1 ? "s" : ""}, ${info.imports.reduce(
        (n, m) => n + m.functions.length,
        0,
      )} function${info.imports.reduce((n, m) => n + m.functions.length, 0) !== 1 ? "s" : ""}`,
    ],
  ];

  const warnings = getWarnings(info);

  return (
    <div className="p-3">
      <PanelTitle>PE Headers</PanelTitle>

      <div className="grid grid-cols-[140px_1fr] border border-[#2b2b2b] bg-[#151515]">
        {fields.map(([k, v]) => (
          <React.Fragment key={k}>
            <div className="px-3 py-[6px] text-[11px] text-[#a6a6a6] border-b border-[#2b2b2b] bg-[#191919]">
              {k}
            </div>
            <div className="px-3 py-[6px] text-[11px] text-[#f2f2f2] border-b border-[#2b2b2b] font-[Cascadia_Code,Consolas,monospace] min-w-0 overflow-hidden text-ellipsis whitespace-nowrap">
              {v}
            </div>
          </React.Fragment>
        ))}
      </div>

      {warnings.length > 0 && (
        <>
          <PanelTitle className="mt-4">Warnings</PanelTitle>

          <div className="border border-[#5c4a1f] bg-[#201a0d]">
            {warnings.map((warning) => (
              <div
                key={warning}
                className="px-3 py-2 text-[12px] text-[#f9e2af] border-b border-[#5c4a1f] last:border-b-0"
              >
                {warning}
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
}

function Sections({ sections }: { sections: PeSection[] }) {
  return (
    <div className="p-3">
      <PanelTitle>Sections ({sections.length})</PanelTitle>

      <div className="border border-[#2b2b2b] overflow-auto">
        <table className="w-full border-collapse text-[11px]">
          <thead className="sticky top-0 z-10 bg-[#191919]">
            <tr className="[&_th]:h-[28px] [&_th]:px-2 [&_th]:border-b [&_th]:border-[#2b2b2b] [&_th]:text-left [&_th]:font-normal [&_th]:text-[#a6a6a6]">
              <th>Name</th>
              <th>Virtual Address</th>
              <th>Virtual Size</th>
              <th>Raw Size</th>
              <th>Permissions</th>
              <th>Notes</th>
            </tr>
          </thead>

          <tbody>
            {sections.map((s, i) => {
              const flags = sectionFlags(s);
              const risky = s.readable && s.writable && s.executable;

              return (
                <tr
                  key={`${s.name}-${i}`}
                  className="border-b border-[#242424] hover:bg-[rgba(255,255,255,0.06)]"
                >
                  <td className="h-[30px] px-2 text-[#f2f2f2] font-[Cascadia_Code,Consolas,monospace]">
                    {s.name}
                  </td>
                  <td className="px-2 text-[#d6d6d6] font-[Cascadia_Code,Consolas,monospace]">
                    {hex(s.virtual_address, 8)}
                  </td>
                  <td className="px-2 text-[#d6d6d6] font-[Cascadia_Code,Consolas,monospace]">
                    {hex(s.virtual_size, 8)}
                  </td>
                  <td className="px-2 text-[#d6d6d6] font-[Cascadia_Code,Consolas,monospace]">
                    {hex(s.raw_size, 8)}
                  </td>
                  <td className={`px-2 font-[Cascadia_Code,Consolas,monospace] tracking-[0.12em] ${flagsClass(flags)}`}>
                    {flags}
                  </td>
                  <td className="px-2 text-[11px]">
                    {risky ? (
                      <span className="text-[#ffb3bd]">RWX section</span>
                    ) : s.executable && s.writable ? (
                      <span className="text-[#ffb3bd]">Writable code</span>
                    ) : s.raw_size === 0 ? (
                      <span className="text-[#a6a6a6]">No raw data</span>
                    ) : (
                      <span className="text-[#666]">—</span>
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function Imports({ imports }: { imports: PeImportModule[] }) {
  const [query, setQuery] = useState("");

  const total = imports.reduce((n, m) => n + m.functions.length, 0);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();

    if (!q) return imports;

    return imports
      .map((mod) => {
        const dllMatches = mod.dll.toLowerCase().includes(q);

        const functions = dllMatches
          ? mod.functions
          : mod.functions.filter((fn) => fn.toLowerCase().includes(q));

        return {
          ...mod,
          functions,
        };
      })
      .filter((mod) => mod.functions.length > 0 || mod.dll.toLowerCase().includes(q));
  }, [imports, query]);

  return (
    <div className="p-3">
      <div className="flex items-center justify-between gap-3 mb-2">
        <PanelTitle className="mb-0">
          Imports — {imports.length} DLL{imports.length !== 1 ? "s" : ""}, {total} function
          {total !== 1 ? "s" : ""}
        </PanelTitle>

        <input
          className="w-[220px] h-[28px] px-2 rounded-none border border-[#4a4a4a] bg-[#111111] text-[#f2f2f2] text-[12px] outline-none hover:border-[#666] focus:border-[#0078d7]"
          placeholder="Search imports"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          spellCheck={false}
        />
      </div>

      <div className="border border-[#2b2b2b] bg-[#151515]">
        {filtered.length === 0 && (
          <div className="px-3 py-4 text-[12px] text-[#a6a6a6]">
            No imports matched your search.
          </div>
        )}

        {filtered.map((mod, i) => (
          <details key={`${mod.dll}-${i}`} className="group border-b border-[#242424] last:border-b-0">
            <summary className="h-[32px] px-3 cursor-default list-none flex items-center gap-[7px] text-[12px] text-[#f2f2f2] hover:bg-[rgba(255,255,255,0.07)] [&::-webkit-details-marker]:hidden">
              <span className="w-3 text-[10px] text-[#a6a6a6] transition-transform group-open:rotate-90">
                ▶
              </span>

              <span className="font-[Cascadia_Code,Consolas,monospace] text-[#8ecbff]">
                {mod.dll}
              </span>

              <span className="text-[#a6a6a6]">
                {mod.functions.length}
              </span>
            </summary>

            <ul className="list-none m-0 px-0 py-1 bg-[#111111] columns-2 max-[680px]:columns-1">
              {mod.functions.map((fn, j) => (
                <li
                  key={`${fn}-${j}`}
                  className="break-inside-avoid px-8 py-[2px] text-[11px] text-[#d6d6d6] font-[Cascadia_Code,Consolas,monospace] hover:bg-[rgba(255,255,255,0.06)]"
                >
                  {fn}
                </li>
              ))}
            </ul>
          </details>
        ))}
      </div>
    </div>
  );
}

function PanelTitle({
  children,
  className = "",
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div className={`mb-2 text-[11px] uppercase tracking-[0.06em] text-[#a6a6a6] ${className}`}>
      {children}
    </div>
  );
}

function getWarnings(info: PeInfo): string[] {
  const warnings: string[] = [];

  const rwxSections = info.sections.filter(
    (s) => s.readable && s.writable && s.executable,
  );

  if (rwxSections.length > 0) {
    warnings.push(
      `Executable contains RWX section${rwxSections.length !== 1 ? "s" : ""}: ${rwxSections
        .map((s) => s.name)
        .join(", ")}`,
    );
  }

  const writableCodeSections = info.sections.filter(
    (s) => s.writable && s.executable && !(s.readable && s.writable && s.executable),
  );

  if (writableCodeSections.length > 0) {
    warnings.push(
      `Executable contains writable executable section${writableCodeSections.length !== 1 ? "s" : ""}: ${writableCodeSections
        .map((s) => s.name)
        .join(", ")}`,
    );
  }

  if (info.entry_point_rva === 0 && !info.is_dll) {
    warnings.push("Entry point RVA is zero.");
  }

  if (info.imports.length === 0) {
    warnings.push("No import table was found.");
  }

  return warnings;
}

function sectionFlags(section: PeSection): string {
  return [
    section.readable ? "R" : "-",
    section.writable ? "W" : "-",
    section.executable ? "X" : "-",
  ].join("");
}

function flagsClass(flags: string): string {
  switch (flags) {
    case "RWX":
      return "text-[#ffb3bd]";
    case "R--":
      return "text-[#a6e3a1]";
    case "R-X":
      return "text-[#f9e2af]";
    case "RW-":
      return "text-[#8ecbff]";
    default:
      return "text-[#a6a6a6]";
  }
}

function hex(value: number, pad = 8): string {
  return `0x${value.toString(16).toUpperCase().padStart(pad, "0")}`;
}