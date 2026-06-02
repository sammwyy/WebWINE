import { createWindow } from "./manager.js";
import type { RuntimeBridge } from "../runtime-bridge.js";
import type { PeInfo, PeSection, PeImportModule } from "../worker.js";
import { log } from "../log.js";

export function openPeInspector(path: string, runtime: RuntimeBridge) {
  const fileName = path.split("\\").pop() ?? path;
  const { body, setTitle } = createWindow({ title: fileName, width: 660, height: 520 });

  body.innerHTML = `<div class="pe-loading">Parsing PE headers…</div>`;

  runtime.inspectPe(path).then((info) => {
    setTitle(`${fileName} — ${info.is_dll ? "DLL" : "EXE"} · ${info.machine} · ${info.subsystem}`);
    body.innerHTML = "";
    body.appendChild(buildInspector(info));
    log("pe", `${fileName}: ${info.machine} ${info.subsystem} image_base=0x${info.image_base.toString(16).toUpperCase().padStart(8, "0")} entry=0x${info.entry_point_rva.toString(16).toUpperCase().padStart(8, "0")}`);
  }).catch((err) => {
    body.innerHTML = `<div class="pe-error">Failed to parse PE: ${err}</div>`;
    log("pe", `failed to parse ${fileName}: ${err}`, "error");
  });
}

function buildInspector(info: PeInfo): HTMLElement {
  const root = document.createElement("div");
  root.className = "pe-inspector";

  root.appendChild(buildHeaderGrid(info));
  root.appendChild(buildSections(info.sections));
  root.appendChild(buildImports(info.imports));

  return root;
}

function buildHeaderGrid(info: PeInfo): HTMLElement {
  const grid = document.createElement("div");
  grid.className = "pe-header-grid";

  const fields: [string, string][] = [
    ["Machine",      `${info.machine} (0x${info.machine_id.toString(16).toUpperCase().padStart(4, "0")})`],
    ["Type",         info.is_dll ? "DLL" : (info.is_pe32 ? "PE32 Executable" : "PE32+ Executable")],
    ["Subsystem",    `${info.subsystem} (${info.subsystem_id})`],
    ["Image Base",   `0x${info.image_base.toString(16).toUpperCase().padStart(8, "0")}`],
    ["Entry Point",  `0x${info.entry_point_rva.toString(16).toUpperCase().padStart(8, "0")} (RVA)`],
    ["Image Size",   formatSize(info.size_of_image)],
  ];

  for (const [label, value] of fields) {
    const key = document.createElement("div");
    key.className = "pe-field-key";
    key.textContent = label;

    const val = document.createElement("div");
    val.className = "pe-field-val";
    val.textContent = value;

    grid.append(key, val);
  }

  return grid;
}

function buildSections(sections: PeSection[]): HTMLElement {
  const wrap = document.createElement("div");
  wrap.className = "pe-section-wrap";

  const heading = document.createElement("div");
  heading.className = "pe-section-heading";
  heading.textContent = `Sections (${sections.length})`;
  wrap.appendChild(heading);

  const table = document.createElement("table");
  table.className = "pe-table";

  const thead = document.createElement("thead");
  thead.innerHTML = "<tr><th>Name</th><th>Virt Addr</th><th>Virt Size</th><th>Raw Size</th><th>Flags</th></tr>";
  table.appendChild(thead);

  const tbody = document.createElement("tbody");
  for (const s of sections) {
    const tr = document.createElement("tr");
    const flags = [
      s.readable   ? "R" : "-",
      s.writable   ? "W" : "-",
      s.executable ? "X" : "-",
    ].join("");
    tr.innerHTML = `
      <td class="pe-mono">${escHtml(s.name)}</td>
      <td class="pe-mono">0x${s.virtual_address.toString(16).toUpperCase().padStart(8, "0")}</td>
      <td class="pe-mono">0x${s.virtual_size.toString(16).toUpperCase().padStart(6, "0")}</td>
      <td class="pe-mono">0x${s.raw_size.toString(16).toUpperCase().padStart(6, "0")}</td>
      <td class="pe-flags pe-flags-${flags}">${flags}</td>
    `;
    tbody.appendChild(tr);
  }
  table.appendChild(tbody);
  wrap.appendChild(table);

  return wrap;
}

function buildImports(modules: PeImportModule[]): HTMLElement {
  const wrap = document.createElement("div");
  wrap.className = "pe-import-wrap";

  const total = modules.reduce((n, m) => n + m.functions.length, 0);
  const heading = document.createElement("div");
  heading.className = "pe-section-heading";
  heading.textContent = `Imports — ${modules.length} DLL${modules.length !== 1 ? "s" : ""}, ${total} function${total !== 1 ? "s" : ""}`;
  wrap.appendChild(heading);

  for (const mod of modules) {
    const details = document.createElement("details");
    details.className = "pe-import-dll";

    const summary = document.createElement("summary");
    summary.className = "pe-import-dll-name";
    summary.textContent = `${mod.dll}  (${mod.functions.length})`;
    details.appendChild(summary);

    const list = document.createElement("ul");
    list.className = "pe-import-fn-list";
    for (const fn of mod.functions) {
      const li = document.createElement("li");
      li.textContent = fn;
      list.appendChild(li);
    }
    details.appendChild(list);
    wrap.appendChild(details);
  }

  return wrap;
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function escHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}
