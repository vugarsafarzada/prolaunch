import type { ScriptInfo } from "../types";

interface Props {
  script: ScriptInfo;
  isRunning: boolean;
  onStart: (script: ScriptInfo) => void;
  onStop: (script: ScriptInfo) => void;
}

function ScriptButton({ script, isRunning, onStart, onStop }: Props) {
  return (
    <div className={`script-card ${isRunning ? "running" : ""}`}>
      <div className="script-info">
        <span className="script-name">{script.name}</span>
        <span className="script-command">{script.command}</span>
      </div>
      <div className="script-actions">
        {isRunning ? (
          <button
            className="btn-stop"
            onClick={() => onStop(script)}
            title="Stop"
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <rect x="4" y="4" width="16" height="16" rx="2" />
            </svg>
            Stop
          </button>
        ) : (
          <button
            className="btn-start"
            onClick={() => onStart(script)}
            title="Run"
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <polygon points="5 3 19 12 5 21 5 3" />
            </svg>
            Run
          </button>
        )}
      </div>
    </div>
  );
}

export default ScriptButton;
