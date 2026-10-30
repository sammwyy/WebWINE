/**
 * Task Manager — modelled on the Windows 10 one: Processes (grouped, with the
 * yellow usage heat-map), Performance (live graphs) and Details, plus
 * "Fewer details" mode. Data comes from the guest VM process/heap stats.
 */

import { useCallback, useEffect, useMemo, useState } from "react";

import { useWindowStore } from "@/state/windowStore";
import type { ProcessInfo, RuntimeBridge, SystemMemoryInfo } from "@/core/bridge/runtime-bridge";
import { basename, formatSize } from "@/shared/lib/utils";
import { iconUrl } from "@/shared/lib/personalization";
import { launchGuestPath } from "@/shared/lib/guest-launch";
import { MenuBar, type MenuBarItem } from "@/shared/components/MenuBar";
import { SEPARATOR } from "@/modules/desktop/ContextMenu";
import { ContextMenu } from "@/modules/desktop/ContextMenu";
import { Button, Icon } from "@/shared/ui";
import { showPrompt } from "../prompt/PromptDialog";
import { showMessageBox } from "../message-box/MessageBoxApp";
import { openPeInspector } from "../pe-inspector/PeInspectorApp";

const HISTORY = 60;
const SPEEDS = { high: 500, normal: 1000, low: 4000, paused: 0 } as const;
type Speed = keyof typeof SPEEDS;

export function openTaskManager(runtime: RuntimeBridge) {
  const id = useWindowStore.getState().openWindow({
    title: "Task Manager",
    icon: iconUrl("apps/taskmgr"),
    width: 760,
    height: 560,
    minWidth: 320,
    minHeight: 260,
    content: <div />,
  });
  useWindowStore.getState().setContent(id, <TaskManagerApp runtime={runtime} winId={id} />);
}

type TabId = "processes" | "performance" | "details";

const isAlive = (p: ProcessInfo) => p.state.state !== "exited" && p.state.state !== "crashed";

function stateLabel(state: ProcessInfo["state"]): string {
  switch (state.state) {
    case "created":
      return "Starting";
    case "running":
      return "";
    case "blocked":
      return "Not responding";
    case "waiting_for_input":
      return "Waiting";
    case "exited":
      return `Exited (${state.exit_code})`;
    case "crashed":
      return "Crashed";
    default:
      return "";
  }
}

/** Windows 10 heat-map: the busier the cell, the deeper the yellow. */
function heat(fraction: number): React.CSSProperties {
  const f = Math.max(0, Math.min(1, fraction));
  return { background: `rgba(255, 196, 0, ${0.12 + f * 0.5})` };
}

interface Sample {
  heap: number;
  committed: number;
  mapped: number;
  processes: number;
}

function TaskManagerApp({ runtime, winId }: { runtime: RuntimeBridge; winId: string }) {
  const [tab, setTab] = useState<TabId>("processes");
  const [mem, setMem] = useState<SystemMemoryInfo | null>(null);
  const [history, setHistory] = useState<Sample[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [selectedPid, setSelectedPid] = useState<number | null>(null);
  const [speed, setSpeed] = useState<Speed>("normal");
  const [compact, setCompact] = useState(false);
  const [ctx, setCtx] = useState<{ x: number; y: number; pid: number } | null>(null);

  const refresh = useCallback(async () => {
    try {
      const info = await runtime.getSystemMemory();
      setMem(info);
      setError(null);
      setHistory((h) =>
        [
          ...h,
          {
            heap: info.total_heap_used,
            committed: info.total_heap_committed,
            mapped: info.total_mapped,
            processes: info.processes.filter(isAlive).length,
          },
        ].slice(-HISTORY),
      );
    } catch (e) {
      setError(String(e));
    }
  }, [runtime]);

  useEffect(() => {
    void refresh();
    const ms = SPEEDS[speed];
    if (!ms) return;
    const id = window.setInterval(() => void refresh(), ms);
    return () => window.clearInterval(id);
  }, [refresh, speed]);

  const processes = useMemo(() => mem?.processes ?? [], [mem]);
  const selected = processes.find((p) => p.pid === selectedPid) ?? null;
  const heapLimit = mem?.heap_limit_per_process ?? 0x4000_0000;

  const endTask = async (pid: number | null = selectedPid) => {
    if (pid == null) return;
    const proc = processes.find((p) => p.pid === pid);
    const answer = await showMessageBox({
      kind: "message_box",
      title: "Task Manager",
      text: `Do you want to end ${basename(proc?.path ?? "") || `PID ${pid}`}?\n\nUnsaved data in this program will be lost.`,
      style: 0x01 | 0x30, // MB_OKCANCEL | MB_ICONEXCLAMATION
    });
    if (answer !== "ok") return;
    runtime.killProcess(pid);
    setSelectedPid(null);
    void refresh();
  };

  const runNewTask = async () => {
    const target = await showPrompt({
      title: "Create new task",
      label: "Type the name of a program, folder or document, and WebWINE will open it for you.",
      value: "C:\\",
      okLabel: "OK",
    });
    if (target) void launchGuestPath(target, runtime);
  };

  const menus: MenuBarItem[] = [
    {
      label: "File",
      items: [
        { label: "Run new task", action: () => void runNewTask() },
        SEPARATOR,
        { label: "Exit", action: () => useWindowStore.getState().closeWindow(winId) },
      ],
    },
    {
      label: "Options",
      items: [{ label: "Fewer details", checked: compact, action: () => setCompact((v) => !v) }],
    },
    {
      label: "View",
      items: [
        { label: "Refresh now", action: () => void refresh() },
        {
          label: "Update speed",
          children: (["high", "normal", "low", "paused"] as Speed[]).map((s) => ({
            label: s[0].toUpperCase() + s.slice(1),
            checked: speed === s,
            action: () => setSpeed(s),
          })),
        },
      ],
    },
  ];

  const footer = (
    <div className="tm-footer">
      <span className="ww-link flex items-center gap-1" onClick={() => setCompact((v) => !v)}>
        <span className={`tm-chevron ${compact ? "" : "up"}`}>⌃</span>
        {compact ? "More details" : "Fewer details"}
      </span>
      <Button disabled={!selected || !isAlive(selected)} onClick={() => void endTask()}>
        End task
      </Button>
    </div>
  );

  if (compact) {
    const alive = processes.filter(isAlive);
    return (
      <div className="flex flex-col h-full select-none">
        <div className="ww-listview flex-1" tabIndex={0}>
          <div className="ww-listview-body">
            {alive.length === 0 && <div className="px-3 py-6 ww-muted text-center">No running programs</div>}
            {alive.map((p) => (
              <ProcessName
                key={p.pid}
                proc={p}
                selected={p.pid === selectedPid}
                onSelect={() => setSelectedPid(p.pid)}
                onContextMenu={(e) => setCtx({ x: e.clientX, y: e.clientY, pid: p.pid })}
              />
            ))}
          </div>
        </div>
        {footer}
        {ctx && <ProcessMenu {...ctx} onClose={() => setCtx(null)} onEnd={endTask} runtime={runtime} processes={processes} />}
      </div>
    );
  }

  const totalHeap = mem?.total_heap_used ?? 0;
  const totalMapped = mem?.total_mapped ?? 0;
  const committedPct = heapLimit > 0 ? ((mem?.total_heap_committed ?? 0) / (heapLimit * Math.max(1, processes.length))) * 100 : 0;
  const apps = processes.filter(isAlive);
  const ended = processes.filter((p) => !isAlive(p));

  return (
    <div className="flex flex-col h-full select-none">
      <MenuBar items={menus} />

      <div className="ww-tablist tm-tabs" role="tablist">
        {(
          [
            ["processes", "Processes"],
            ["performance", "Performance"],
            ["details", "Details"],
          ] as const
        ).map(([id, label]) => (
          <button
            key={id}
            type="button"
            role="tab"
            aria-selected={tab === id}
            className={`ww-tab ${tab === id ? "selected" : ""}`}
            onClick={() => setTab(id)}
          >
            {label}
          </button>
        ))}
      </div>

      {error && (
        <div className="px-3 py-2 bg-[var(--ww-danger-bg)] border-b border-[var(--ww-danger-border)] text-[var(--ww-danger)]">
          {error}
        </div>
      )}

      {tab === "processes" && (
        <div className="ww-listview flex-1" tabIndex={0}>
          <div className="ww-listview-header tm-proc-cols tm-tall-header">
            <div className="ww-listview-col sorted">Name</div>
            <div className="ww-listview-col">Status</div>
            <div className="ww-listview-col tm-metric">
              <b>{Math.round(committedPct)}%</b>
              <span>Memory</span>
            </div>
            <div className="ww-listview-col tm-metric">
              <b>{formatSize(totalMapped)}</b>
              <span>Mapped</span>
            </div>
          </div>
          <div className="ww-listview-body" onClick={() => setSelectedPid(null)}>
            {processes.length === 0 && <div className="px-3 py-6 ww-muted text-center">No guest processes running</div>}
            {[
              { title: "Apps", items: apps },
              { title: "Ended processes", items: ended },
            ]
              .filter((g) => g.items.length > 0)
              .map((group) => (
                <div key={group.title}>
                  <div className="ww-listview-group">
                    <span>
                      {group.title} ({group.items.length})
                    </span>
                  </div>
                  {group.items.map((p) => (
                    <div
                      key={p.pid}
                      className={`ww-listview-row tm-proc-cols ${p.pid === selectedPid ? "selected" : ""}`}
                      onClick={(e) => {
                        e.stopPropagation();
                        setSelectedPid(p.pid);
                      }}
                      onDoubleClick={() => {
                        setSelectedPid(p.pid);
                        setTab("details");
                      }}
                      onContextMenu={(e) => {
                        e.preventDefault();
                        setSelectedPid(p.pid);
                        setCtx({ x: e.clientX, y: e.clientY, pid: p.pid });
                      }}
                    >
                      <span className="ww-listview-cell flex items-center gap-2">
                        <Icon icon={iconUrl("shell/default_executable")} size={16} />
                        <span className="truncate" title={p.path}>
                          {basename(p.path) || p.path || "(unknown)"}
                        </span>
                      </span>
                      <span className="ww-listview-cell muted">{stateLabel(p.state)}</span>
                      <span className="ww-listview-cell text-right tabular-nums" style={heat(totalHeap ? p.heap_used / totalHeap : 0)}>
                        {formatSize(p.heap_used ?? 0)}
                      </span>
                      <span
                        className="ww-listview-cell text-right tabular-nums"
                        style={heat(totalMapped ? p.mapped_bytes / totalMapped : 0)}
                      >
                        {formatSize(p.mapped_bytes ?? 0)}
                      </span>
                    </div>
                  ))}
                </div>
              ))}
          </div>
        </div>
      )}

      {tab === "performance" && (
        <Performance history={history} mem={mem} heapLimit={heapLimit} processCount={apps.length} />
      )}

      {tab === "details" && (
        <div className="ww-listview flex-1" tabIndex={0}>
          <div className="ww-listview-header tm-detail-cols">
            {["Name", "PID", "Status", "Image base", "Heap", "Committed", "Regions"].map((h, i) => (
              <div key={h} className={`ww-listview-col ${i === 1 ? "sorted" : ""} ${i > 2 ? "justify-end" : ""}`}>
                {h}
              </div>
            ))}
          </div>
          <div className="ww-listview-body">
            {processes.map((p) => (
              <div
                key={p.pid}
                className={`ww-listview-row tm-detail-cols ${p.pid === selectedPid ? "selected" : ""}`}
                onClick={() => setSelectedPid(p.pid)}
                onContextMenu={(e) => {
                  e.preventDefault();
                  setSelectedPid(p.pid);
                  setCtx({ x: e.clientX, y: e.clientY, pid: p.pid });
                }}
              >
                <span className="ww-listview-cell truncate" title={p.path}>
                  {basename(p.path) || p.path}
                </span>
                <span className="ww-listview-cell tabular-nums">{p.pid}</span>
                <span className="ww-listview-cell muted">{stateLabel(p.state) || "Running"}</span>
                <span className="ww-listview-cell muted text-right tabular-nums">
                  0x{(p.image_base >>> 0).toString(16).toUpperCase().padStart(8, "0")}
                </span>
                <span className="ww-listview-cell text-right tabular-nums">{formatSize(p.heap_used)}</span>
                <span className="ww-listview-cell text-right tabular-nums">{formatSize(p.heap_committed)}</span>
                <span className="ww-listview-cell muted text-right tabular-nums">{p.region_count}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {footer}

      {ctx && <ProcessMenu {...ctx} onClose={() => setCtx(null)} onEnd={endTask} runtime={runtime} processes={processes} />}
    </div>
  );
}

function ProcessName({
  proc,
  selected,
  onSelect,
  onContextMenu,
}: {
  proc: ProcessInfo;
  selected: boolean;
  onSelect: () => void;
  onContextMenu: (e: React.MouseEvent) => void;
}) {
  return (
    <div
      className={`ww-listview-row gap-2 px-2 ${selected ? "selected" : ""}`}
      style={{ height: 26 }}
      onClick={onSelect}
      onContextMenu={(e) => {
        e.preventDefault();
        onSelect();
        onContextMenu(e);
      }}
    >
      <Icon icon={iconUrl("shell/default_executable")} size={16} />
      <span className="truncate">{basename(proc.path) || proc.path}</span>
    </div>
  );
}

function ProcessMenu({
  x,
  y,
  pid,
  processes,
  runtime,
  onEnd,
  onClose,
}: {
  x: number;
  y: number;
  pid: number;
  processes: ProcessInfo[];
  runtime: RuntimeBridge;
  onEnd: (pid: number) => void;
  onClose: () => void;
}) {
  const proc = processes.find((p) => p.pid === pid);
  return (
    <ContextMenu
      x={x}
      y={y}
      onClose={onClose}
      items={[
        { label: "End task", disabled: !proc || !isAlive(proc), action: () => onEnd(pid) },
        SEPARATOR,
        { label: "Properties (PE inspector)", disabled: !proc?.path, action: () => proc && void openPeInspector(proc.path, runtime) },
      ]}
    />
  );
}

/* performance */

const COLORS = {
  heap: "#8b12ae",
  committed: "#17a2b8",
  mapped: "#4da60c",
  processes: "#0078d7",
};

function Graph({ values, max, color, height = 120 }: { values: number[]; max: number; color: string; height?: number }) {
  const w = 300;
  const pts = values.map((v, i) => {
    const x = w - (values.length - 1 - i) * (w / (HISTORY - 1));
    const y = height - (max > 0 ? Math.min(1, v / max) : 0) * height;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  });
  const area = pts.length ? `M${pts[0].split(",")[0]},${height} L${pts.join(" L")} L${w},${height} Z` : "";
  return (
    <svg className="tm-graph" viewBox={`0 0 ${w} ${height}`} preserveAspectRatio="none" style={{ borderColor: color }}>
      <defs>
        <pattern id={`grid-${color.slice(1)}`} width={w / 10} height={height / 5} patternUnits="userSpaceOnUse">
          <path d={`M ${w / 10} 0 L 0 0 0 ${height / 5}`} fill="none" stroke={color} strokeOpacity="0.15" strokeWidth="1" />
        </pattern>
      </defs>
      <rect width={w} height={height} fill={`url(#grid-${color.slice(1)})`} />
      {area && <path d={area} fill={color} fillOpacity="0.15" />}
      {pts.length > 1 && <polyline points={pts.join(" ")} fill="none" stroke={color} strokeWidth="1.5" vectorEffect="non-scaling-stroke" />}
    </svg>
  );
}

function Performance({
  history,
  mem,
  heapLimit,
  processCount,
}: {
  history: Sample[];
  mem: SystemMemoryInfo | null;
  heapLimit: number;
  processCount: number;
}) {
  const [resource, setResource] = useState<keyof Sample>("heap");
  const limit = heapLimit * Math.max(1, processCount);
  const peak = (key: keyof Sample) => Math.max(1, ...history.map((s) => s[key]));
  const last = history.at(-1);

  const resources: { id: keyof Sample; title: string; value: string; max: number }[] = [
    { id: "heap", title: "Heap", value: formatSize(last?.heap ?? 0), max: peak("heap") * 1.25 },
    { id: "committed", title: "Committed", value: `${formatSize(last?.committed ?? 0)} / ${formatSize(limit)}`, max: Math.max(peak("committed") * 1.25, 1) },
    { id: "mapped", title: "Mapped memory", value: formatSize(last?.mapped ?? 0), max: peak("mapped") * 1.25 },
    { id: "processes", title: "Processes", value: String(last?.processes ?? 0), max: Math.max(4, peak("processes") + 1) },
  ];
  const current = resources.find((r) => r.id === resource)!;

  return (
    <div className="flex flex-1 min-h-0">
      <div className="tm-perf-list">
        {resources.map((r) => (
          <button
            key={r.id}
            type="button"
            className={`tm-perf-item ${r.id === resource ? "selected" : ""}`}
            onClick={() => setResource(r.id)}
          >
            <Graph values={history.map((s) => s[r.id])} max={r.max} color={COLORS[r.id]} height={40} />
            <span className="flex flex-col min-w-0">
              <span className="text-[14px]">{r.title}</span>
              <span className="ww-muted truncate">{r.value}</span>
            </span>
          </button>
        ))}
      </div>

      <div className="flex-1 min-w-0 overflow-auto p-4 flex flex-col gap-2">
        <div className="flex items-end justify-between">
          <span className="text-[24px] font-light">{current.title}</span>
          <span className="text-[14px] ww-muted">{current.value}</span>
        </div>
        <div className="flex justify-between ww-muted text-[11px]">
          <span>Usage over the last {HISTORY} samples</span>
          <span>{current.id === "processes" ? current.max : formatSize(current.max)}</span>
        </div>
        <Graph values={history.map((s) => s[current.id])} max={current.max} color={COLORS[current.id]} height={180} />
        <div className="flex justify-between ww-muted text-[11px]">
          <span>{HISTORY} samples</span>
          <span>0</span>
        </div>

        <div className="tm-stats">
          <div>
            <span className="ww-muted">In use</span>
            <b>{formatSize(mem?.total_heap_used ?? 0)}</b>
          </div>
          <div>
            <span className="ww-muted">Free list</span>
            <b>{formatSize(mem?.total_heap_free ?? 0)}</b>
          </div>
          <div>
            <span className="ww-muted">Committed</span>
            <b>{formatSize(mem?.total_heap_committed ?? 0)}</b>
          </div>
          <div>
            <span className="ww-muted">Mapped</span>
            <b>{formatSize(mem?.total_mapped ?? 0)}</b>
          </div>
          <div>
            <span className="ww-muted">Processes</span>
            <b>{processCount}</b>
          </div>
          <div>
            <span className="ww-muted">Heap limit / process</span>
            <b>{formatSize(heapLimit)}</b>
          </div>
        </div>
        <p className="ww-muted text-[11px] leading-relaxed">
          Layout: image @ 0x00400000 · heap 0x10000000 → 0x50000000 (~1 GiB growable) · DLLs @ 0x50000000 · stack @
          0x6FF00000.
        </p>
      </div>
    </div>
  );
}
