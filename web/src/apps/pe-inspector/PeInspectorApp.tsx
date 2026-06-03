import "./PeInspectorApp.css";
import { useEffect, useState } from "react";
import { useWindowStore } from "../../stores/useWindowStore.js";
import type { RuntimeBridge } from "../../lib/runtime-bridge.js";
import type { PeInfo, PeSection, PeImportModule } from "../../lib/worker.js";
import { basename, formatSize, escHtml } from "../../lib/utils.js";
import { log } from "../../stores/useLogStore.js";

import { useThemeStore } from "../../stores/useThemeStore.js";
import { resolveIcon } from "../../lib/icon-resolver.js";

export async function openPeInspector(path: string, runtime: RuntimeBridge) {
  const name = basename(path);
  const theme = useThemeStore.getState().theme;
  
  const resolved = await resolveIcon(
    { name, path, kind: "file", size: 0 },
    runtime
  );
  const icon = resolved?.src || `/themes/${theme}/icons/shell/default_executable.webp`;

  let winId = "";
  winId = useWindowStore.getState().openWindow({
    title: name,
    icon,
    width: 660,
    height: 520,
    content: <PeInspectorApp path={path} name={name} runtime={runtime} winId={() => winId} />,
  });
}

function PeInspectorApp({
  path,
  name,
  runtime,
  winId,
}: {
  path: string;
  name: string;
  runtime: RuntimeBridge;
  winId?: () => string;
}) {
  const [info, setInfo] = useState<PeInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    runtime
      .inspectPe(path)
      .then((res) => {
        setInfo(res);
        log(
          "pe",
          `${name}: ${res.machine} ${res.subsystem} image_base=0x${res.image_base.toString(16).toUpperCase().padStart(8, "0")} entry=0x${res.entry_point_rva.toString(16).toUpperCase().padStart(8, "0")}`,
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
        setError(String(err));
        log("pe", `failed to parse ${name}: ${err}`, "error");
      });
  }, [path, name, runtime, winId]);

  if (error) {
    return <div className="pe-error">Failed to parse PE: {error}</div>;
  }

  if (!info) {
    return <div className="pe-loading">Parsing PE headers…</div>;
  }

  return (
    <div className="pe-inspector">
      <HeaderGrid info={info} />
      <Sections sections={info.sections} />
      <Imports imports={info.imports} />
    </div>
  );
}

function HeaderGrid({ info }: { info: PeInfo }) {
  const fields = [
    [
      "Machine",
      `${info.machine} (0x${info.machine_id.toString(16).toUpperCase().padStart(4, "0")})`,
    ],
    [
      "Type",
      info.is_dll
        ? "DLL"
        : info.is_pe32
          ? "PE32 Executable"
          : "PE32+ Executable",
    ],
    ["Subsystem", `${info.subsystem} (${info.subsystem_id})`],
    [
      "Image Base",
      `0x${info.image_base.toString(16).toUpperCase().padStart(8, "0")}`,
    ],
    [
      "Entry Point",
      `0x${info.entry_point_rva.toString(16).toUpperCase().padStart(8, "0")} (RVA)`,
    ],
    ["Image Size", formatSize(info.size_of_image)],
  ];

  return (
    <div className="pe-header-grid">
      {fields.map(([k, v]) => (
        <React.Fragment key={k}>
          <div className="pe-field-key">{k}</div>
          <div className="pe-field-val">{v}</div>
        </React.Fragment>
      ))}
    </div>
  );
}

// React isn't imported explicitly in HeaderGrid because of new JSX transform, but React.Fragment needs it. Let's fix that.
import React from "react";

function Sections({ sections }: { sections: PeSection[] }) {
  return (
    <div className="pe-section-wrap">
      <div className="pe-section-heading">Sections ({sections.length})</div>
      <table className="pe-table">
        <thead>
          <tr>
            <th>Name</th>
            <th>Virt Addr</th>
            <th>Virt Size</th>
            <th>Raw Size</th>
            <th>Flags</th>
          </tr>
        </thead>
        <tbody>
          {sections.map((s, i) => {
            const flags = [
              s.readable ? "R" : "-",
              s.writable ? "W" : "-",
              s.executable ? "X" : "-",
            ].join("");
            return (
              <tr key={i}>
                <td className="pe-mono">{s.name}</td>
                <td className="pe-mono">
                  0x
                  {s.virtual_address.toString(16).toUpperCase().padStart(8, "0")}
                </td>
                <td className="pe-mono">
                  0x{s.virtual_size.toString(16).toUpperCase().padStart(6, "0")}
                </td>
                <td className="pe-mono">
                  0x{s.raw_size.toString(16).toUpperCase().padStart(6, "0")}
                </td>
                <td className={`pe-flags pe-flags-${flags}`}>{flags}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

function Imports({ imports }: { imports: PeImportModule[] }) {
  const total = imports.reduce((n, m) => n + m.functions.length, 0);

  return (
    <div className="pe-import-wrap">
      <div className="pe-section-heading">
        Imports — {imports.length} DLL{imports.length !== 1 && "s"},{" "}
        {total} function{total !== 1 && "s"}
      </div>
      {imports.map((mod, i) => (
        <details key={i} className="pe-import-dll">
          <summary className="pe-import-dll-name">
            {mod.dll} ({mod.functions.length})
          </summary>
          <ul className="pe-import-fn-list">
            {mod.functions.map((fn, j) => (
              <li key={j}>{fn}</li>
            ))}
          </ul>
        </details>
      ))}
    </div>
  );
}
