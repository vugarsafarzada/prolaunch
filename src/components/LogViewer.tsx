import { useEffect, useRef } from "react";
import type { LogLine } from "../types";

interface Props {
  logs: LogLine[];
  activeScript: string | null;
  isRunning: boolean;
  onClear: () => void;
  onReRun: () => void;
}

function logTone(log: LogLine) {
  const text = log.text.toLowerCase();

  if (
    text.includes("warning") ||
    text.includes("warn") ||
    text.includes("deprecated")
  ) {
    return "warning";
  }

  if (
    text.startsWith("error:") ||
    text.includes(" failed") ||
    text.includes("failed ") ||
    text.includes("traceback") ||
    text.includes("exception") ||
    text.includes("fatal") ||
    text.includes("panic") ||
    /exit(ed)? with code [1-9]/.test(text)
  ) {
    return "error";
  }

  if (
    text.includes("success") ||
    text.includes("completed") ||
    text.includes("started with") ||
    text.includes("created successfully") ||
    text.includes("process exited with code 0")
  ) {
    return "success";
  }

  return log.isError ? "warning" : "default";
}

function LogViewer({ logs, activeScript, isRunning, onClear, onReRun }: Props) {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  return (
    <div className="log-viewer">
      <div className="log-header">
        <span className="log-title">Output</span>
        <div className="log-header-actions">
          {activeScript && isRunning && (
            <button className="btn-rerun" onClick={onReRun} title="Restart script">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <polyline points="23 4 23 10 17 10" />
                <polyline points="1 20 1 14 7 14" />
                <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
              </svg>
              Re-run
            </button>
          )}
          {logs.length > 0 && (
            <button className="btn-clear" onClick={onClear}>
              Clear
            </button>
          )}
        </div>
      </div>
      <div className="log-content">
        {logs.length === 0 ? (
          <div className="log-empty">No output yet. Run a script to see logs here.</div>
        ) : (
          logs.map((log, i) => (
            <div key={i} className={`log-line ${logTone(log)}`}>
              <span className="log-line-number">{i + 1}</span>
              <span className="log-line-text">{log.text}</span>
            </div>
          ))
        )}
        <div ref={bottomRef} />
      </div>
    </div>
  );
}

export default LogViewer;
