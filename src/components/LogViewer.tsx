import { useEffect, useRef } from "react";
import type { LogLine } from "../types";

interface Props {
  logs: LogLine[];
  onClear: () => void;
}

function LogViewer({ logs, onClear }: Props) {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  return (
    <div className="log-viewer">
      <div className="log-header">
        <span className="log-title">Output</span>
        {logs.length > 0 && (
          <button className="btn-clear" onClick={onClear}>
            Clear
          </button>
        )}
      </div>
      <div className="log-content">
        {logs.length === 0 ? (
          <div className="log-empty">No output yet. Run a script to see logs here.</div>
        ) : (
          logs.map((log, i) => (
            <div key={i} className={`log-line ${log.isError ? "error" : ""}`}>
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
