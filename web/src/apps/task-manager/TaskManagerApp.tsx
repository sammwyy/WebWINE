/**
 * Task Manager — Processes + Memory tabs over guest VM process/heap stats.
 */

import { useCallback, useEffect, useMemo, useState } from "react";

import { useWindowStore } from "@/state/windowStore";
import type { ProcessInfo, RuntimeBridge, SystemMemoryInfo } from "@/core/bridge/runtime-bridge";
import { basename, formatSize } from "@/shared/lib/utils";

const ICON = `${import.meta.env.BASE_URL}theme/icons/apps/taskmgr.webp`;
const REFRESH_MS = 1000;

export function openTaskManager(runtime: RuntimeBridge) {
  const id = useWindowStore.getState().openWindow({
    title: "Task Manager",
    icon: ICON,
    width: 720,
    height: 520,
    content: <div />,
  });
  useWindowStore.getState().setContent(id, <TaskManagerApp runtime={runtime} />);
}

type TabId = "processes" | "memory";

function stateLabel(state: ProcessInfo["state"]): string {
  switch (state.state) {
    case "created":
      return "Created";
    case "running":
      return "Running";
    case "blocked":
      return "Blocked";
    case "waiting_for_input":
      return "Waiting";
    case "exited":
      return `Exited (${state.exit_code})`;
    case "crashed":
      return "Crashed";
    default:
      return "Unknown";
  }
}

function stateTone(state: ProcessInfo["state"]): string {
  switch (state.state) {
    case "running":
      return "text-emerald-400";
    case "waiting_for_input":
    case "blocked":
      return "text-amber-300";
    case "exited":
      return "text-[var(--text-muted)]";
    case "crashed":
      return "text-red-400";
    default:
      return "text-[var(--text-muted)]";
  }
}

function MemBar({
  label,
  used,
  total,
  color = "var(--accent)",
}: {
  label: string;
  used: number;
  total: number;
  color?: string;
}) {
  const pct = total > 0 ? Math.min(100, (used / total) * 100) : 0;
  return (
    <div className="flex flex-col gap-1">
      <div className="flex justify-between text-[12px]">
        <span className="text-[var(--text-muted)]">{label}</span>
        <span className="tabular-nums">
          {formatSize(used)}
          {total > 0 ? (
            <span className="text-[var(--text-muted)]"> / {formatSize(total)}</span>
          ) : null}
          <span className="text-[var(--text-muted)] ml-1">({pct.toFixed(1)}%)</span>
        </span>
      </div>
      <div className="h-2.5 rounded-sm bg-[rgba(255,255,255,0.08)] overflow-hidden">
        <div
          className="h-full rounded-sm transition-[width] duration-300"
          style={{ width: `${pct}%`, background: color }}
        />
      </div>
    </div>
  );
}

function StatCard({ label, value, hint }: { label: string; value: string; hint?: string }) {
  return (
    <div className="rounded border border-[rgba(255,255,255,0.08)] bg-[rgba(255,255,255,0.03)] px-3 py-2 min-w-0">
      <div className="text-[11px] uppercase tracking-wide text-[var(--text-muted)]">{label}</div>
      <div className="text-[16px] font-semibold tabular-nums mt-0.5 truncate" title={value}>
        {value}
      </div>
      {hint ? <div className="text-[11px] text-[var(--text-muted)] mt-0.5 truncate">{hint}</div> : null}
    </div>
  );
}

function TaskManagerApp({ runtime }: { runtime: RuntimeBridge }) {
  const [tab, setTab] = useState<TabId>("processes");
  const [mem, setMem] = useState<SystemMemoryInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selectedPid, setSelectedPid] = useState<number | null>(null);
  const [paused, setPaused] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const info = await runtime.getSystemMemory();
      setMem(info);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, [runtime]);

  useEffect(() => {
    void refresh();
    if (paused) return;
    const id = window.setInterval(() => void refresh(), REFRESH_MS);
    return () => window.clearInterval(id);
  }, [refresh, paused]);

  const processes = mem?.processes ?? [];
  const selected = useMemo(
    () => processes.find((p) => p.pid === selectedPid) ?? null,
    [processes, selectedPid],
  );

  const onEndTask = () => {
    if (selectedPid == null) return;
    if (!window.confirm(`End process PID ${selectedPid}?`)) return;
    runtime.killProcess(selectedPid);
    setSelectedPid(null);
    void refresh();
  };

  const heapLimit = mem?.heap_limit_per_process ?? 0x4000_0000;

  return (
    <div className="flex flex-col h-full bg-[var(--window-bg,#111)] text-[var(--text,#f2f2f2)] select-none">
      {/* Tabs */}
      <div className="flex items-center gap-0 border-b border-[rgba(255,255,255,0.1)] px-2 pt-1">
        {(
          [
            ["processes", "Processes"],
            ["memory", "Memory"],
          ] as const
        ).map(([id, label]) => (
          <button
            key={id}
            type="button"
            className={`px-3 py-1.5 text-[13px] border-b-2 -mb-px transition-colors ${
              tab === id
                ? "border-[var(--accent)] text-white"
                : "border-transparent text-[var(--text-muted)] hover:text-white"
            }`}
            onClick={() => setTab(id)}
          >
            {label}
          </button>
        ))}
        <div className="flex-1" />
        <button
          type="button"
          className="px-2 py-1 text-[12px] rounded hover:bg-[rgba(255,255,255,0.08)] text-[var(--text-muted)]"
          onClick={() => setPaused((p) => !p)}
          title={paused ? "Resume auto-refresh" : "Pause auto-refresh"}
        >
          {paused ? "Resume" : "Pause"}
        </button>
        <button
          type="button"
          className="px-2 py-1 text-[12px] rounded hover:bg-[rgba(255,255,255,0.08)] text-[var(--text-muted)]"
          onClick={() => void refresh()}
        >
          Refresh
        </button>
      </div>

      {error ? (
        <div className="m-3 p-2 text-[12px] text-red-300 border border-red-900/50 rounded bg-red-950/30">
          {error}
        </div>
      ) : null}

      {tab === "processes" ? (
        <div className="flex flex-col flex-1 min-h-0">
          <div className="flex-1 min-h-0 overflow-auto">
            <table className="w-full text-[12px] border-collapse">
              <thead className="sticky top-0 bg-[var(--window-bg,#111)] z-10">
                <tr className="text-left text-[var(--text-muted)] border-b border-[rgba(255,255,255,0.08)]">
                  <th className="px-2 py-1.5 font-medium w-14">PID</th>
                  <th className="px-2 py-1.5 font-medium">Name</th>
                  <th className="px-2 py-1.5 font-medium w-24">Status</th>
                  <th className="px-2 py-1.5 font-medium w-24 text-right">Heap</th>
                  <th className="px-2 py-1.5 font-medium w-24 text-right">Mapped</th>
                  <th className="px-2 py-1.5 font-medium w-20 text-right">Blocks</th>
                </tr>
              </thead>
              <tbody>
                {processes.length === 0 ? (
                  <tr>
                    <td colSpan={6} className="px-3 py-8 text-center text-[var(--text-muted)]">
                      No guest processes running
                    </td>
                  </tr>
                ) : (
                  processes.map((p) => {
                    const sel = p.pid === selectedPid;
                    return (
                      <tr
                        key={p.pid}
                        className={`border-b border-[rgba(255,255,255,0.04)] cursor-pointer ${
                          sel
                            ? "bg-[rgba(0,120,215,0.28)]"
                            : "hover:bg-[rgba(255,255,255,0.05)]"
                        }`}
                        onClick={() => setSelectedPid(p.pid)}
                        onDoubleClick={() => setTab("memory")}
                      >
                        <td className="px-2 py-1 tabular-nums text-[var(--text-muted)]">{p.pid}</td>
                        <td className="px-2 py-1 truncate max-w-[220px]" title={p.path}>
                          {basename(p.path) || p.path || "(unknown)"}
                        </td>
                        <td className={`px-2 py-1 ${stateTone(p.state)}`}>{stateLabel(p.state)}</td>
                        <td className="px-2 py-1 text-right tabular-nums">
                          {formatSize(p.heap_used ?? 0)}
                        </td>
                        <td className="px-2 py-1 text-right tabular-nums">
                          {formatSize(p.mapped_bytes ?? 0)}
                        </td>
                        <td className="px-2 py-1 text-right tabular-nums text-[var(--text-muted)]">
                          {p.heap_blocks ?? 0}
                        </td>
                      </tr>
                    );
                  })
                )}
              </tbody>
            </table>
          </div>

          {selected ? (
            <div className="border-t border-[rgba(255,255,255,0.08)] px-3 py-2 text-[11px] text-[var(--text-muted)] grid grid-cols-2 gap-x-4 gap-y-0.5">
              <div className="col-span-2 truncate text-[12px] text-[var(--text)]" title={selected.path}>
                {selected.path}
              </div>
              <div>
                Heap used: <span className="text-[var(--text)]">{formatSize(selected.heap_used)}</span>
                {" · "}
                free list: {formatSize(selected.heap_free)} ({selected.heap_free_blocks} blocks)
              </div>
              <div>
                Committed: <span className="text-[var(--text)]">{formatSize(selected.heap_committed)}</span>
                {" / "}
                {formatSize(selected.heap_limit)}
              </div>
              <div>
                Mapped: <span className="text-[var(--text)]">{formatSize(selected.mapped_bytes)}</span>
                {" · "}
                {selected.region_count} regions
              </div>
              <div>
                Image base: 0x{(selected.image_base >>> 0).toString(16).toUpperCase().padStart(8, "0")}
              </div>
            </div>
          ) : null}

          <div className="flex items-center justify-between px-2 py-1.5 border-t border-[rgba(255,255,255,0.1)]">
            <span className="text-[11px] text-[var(--text-muted)]">
              {processes.length} process{processes.length === 1 ? "" : "es"}
              {mem ? ` · total heap ${formatSize(mem.total_heap_used)}` : ""}
            </span>
            <button
              type="button"
              disabled={selectedPid == null}
              className="px-3 py-1 text-[12px] rounded border border-[rgba(255,255,255,0.12)] disabled:opacity-40 hover:enabled:bg-[rgba(255,80,80,0.2)] hover:enabled:border-red-500/40"
              onClick={onEndTask}
            >
              End task
            </button>
          </div>
        </div>
      ) : (
        <div className="flex-1 min-h-0 overflow-auto p-3 flex flex-col gap-4">
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
            <StatCard
              label="Heap used"
              value={formatSize(mem?.total_heap_used ?? 0)}
              hint="Live allocations"
            />
            <StatCard
              label="Free list"
              value={formatSize(mem?.total_heap_free ?? 0)}
              hint="Reusable without bump"
            />
            <StatCard
              label="Heap committed"
              value={formatSize(mem?.total_heap_committed ?? 0)}
              hint={`Limit ~${formatSize(heapLimit)} / proc`}
            />
            <StatCard
              label="Mapped total"
              value={formatSize(mem?.total_mapped ?? 0)}
              hint="All VA regions"
            />
          </div>

          <div className="rounded border border-[rgba(255,255,255,0.08)] bg-[rgba(255,255,255,0.02)] p-3 flex flex-col gap-3">
            <div className="text-[12px] font-medium">Guest memory layout</div>
            <MemBar
              label="Process heap capacity (per process)"
              used={mem?.total_heap_committed ?? 0}
              total={heapLimit * Math.max(1, processes.length)}
              color="#0078d7"
            />
            <MemBar
              label="Heap in use vs committed"
              used={mem?.total_heap_used ?? 0}
              total={Math.max(mem?.total_heap_committed ?? 1, 1)}
              color="#16a34a"
            />
            <MemBar
              label="Free-list reuse pool"
              used={mem?.total_heap_free ?? 0}
              total={Math.max((mem?.total_heap_used ?? 0) + (mem?.total_heap_free ?? 0), 1)}
              color="#ca8a04"
            />
            <p className="text-[11px] text-[var(--text-muted)] leading-relaxed">
              Layout: image @ 0x00400000 · heap 0x10000000 → 0x50000000 (~1 GiB growable) ·
              DLLs @ 0x50000000 · stack @ 0x6FF00000. HeapAlloc/malloc use a free-list so
              games that allocate and free assets do not exhaust the bump high-water mark.
            </p>
          </div>

          <div className="rounded border border-[rgba(255,255,255,0.08)] overflow-hidden">
            <div className="px-3 py-1.5 text-[12px] font-medium border-b border-[rgba(255,255,255,0.08)] bg-[rgba(255,255,255,0.03)]">
              Per-process breakdown
            </div>
            {processes.length === 0 ? (
              <div className="px-3 py-6 text-center text-[12px] text-[var(--text-muted)]">
                No processes
              </div>
            ) : (
              <table className="w-full text-[12px]">
                <thead>
                  <tr className="text-left text-[var(--text-muted)] border-b border-[rgba(255,255,255,0.06)]">
                    <th className="px-2 py-1 font-medium">Process</th>
                    <th className="px-2 py-1 font-medium text-right">Used</th>
                    <th className="px-2 py-1 font-medium text-right">Free</th>
                    <th className="px-2 py-1 font-medium text-right">Committed</th>
                    <th className="px-2 py-1 font-medium text-right">Mapped</th>
                    <th className="px-2 py-1 font-medium text-right">Regions</th>
                  </tr>
                </thead>
                <tbody>
                  {processes.map((p) => (
                    <tr
                      key={p.pid}
                      className="border-b border-[rgba(255,255,255,0.04)] hover:bg-[rgba(255,255,255,0.04)]"
                    >
                      <td className="px-2 py-1 truncate max-w-[160px]" title={p.path}>
                        <span className="text-[var(--text-muted)] mr-1">{p.pid}</span>
                        {basename(p.path)}
                      </td>
                      <td className="px-2 py-1 text-right tabular-nums">{formatSize(p.heap_used)}</td>
                      <td className="px-2 py-1 text-right tabular-nums">{formatSize(p.heap_free)}</td>
                      <td className="px-2 py-1 text-right tabular-nums">
                        {formatSize(p.heap_committed)}
                      </td>
                      <td className="px-2 py-1 text-right tabular-nums">
                        {formatSize(p.mapped_bytes)}
                      </td>
                      <td className="px-2 py-1 text-right tabular-nums text-[var(--text-muted)]">
                        {p.region_count}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
