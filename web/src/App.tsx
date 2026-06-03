import { useEffect, useRef } from "react";
import { useRuntimeStore } from "./state/runtimeStore";
import { useLogStore } from "./state/logStore";
import { Desktop } from "./modules/desktop/Desktop";
import { Taskbar } from "./modules/taskbar/Taskbar";

export function App() {
  const { ready, init } = useRuntimeStore();

  const fileInputRef = useRef<HTMLInputElement>(null);
  const folderInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    // Only initialize the runtime once
    if (!ready) {
      init().catch(console.error);
    }
  }, [ready, init]);

  return (
    <>
      <Desktop fileInputRef={fileInputRef} folderInputRef={folderInputRef} />
      <Taskbar fileInputRef={fileInputRef} folderInputRef={folderInputRef} />
      <LogPanel />
    </>
  );
}

function LogPanel() {
  const { entries, clear } = useLogStore();
  const outputRef = useRef<HTMLDivElement>(null);

  // Auto-scroll on new logs
  useEffect(() => {
    if (outputRef.current) {
      outputRef.current.scrollTop = outputRef.current.scrollHeight;
    }
  }, [entries]);

  return (
    <div id="log-panel">
      <div id="log-panel-header">
        <span>System Log</span>
        <button id="log-clear-btn" type="button" onClick={clear}>
          Clear
        </button>
      </div>
      <div id="log-output" ref={outputRef}>
        {entries.map((entry) => (
          <div key={entry.key} className={`log-line log-${entry.level}`}>
            <span className="log-target">[{entry.target}]</span>
            <span className="log-message"> {entry.message}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
