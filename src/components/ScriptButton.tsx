import type { ScriptInfo } from "../types";

interface Props {
  script: ScriptInfo;
  isRunning: boolean;
  isPinned: boolean;
  onStart: (script: ScriptInfo) => void;
  onStop: (script: ScriptInfo) => void;
  onTogglePin: (script: ScriptInfo) => void;
}

function ScriptButton({ script, isRunning, isPinned, onStart, onStop, onTogglePin }: Props) {
  return (
    <div className={`script-card ${isRunning ? "running" : ""} ${isPinned ? "pinned" : ""}`}>
      <button
        className="btn-pin"
        onClick={() => onTogglePin(script)}
        title={isPinned ? "Unpin" : "Pin to top"}
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill={isPinned ? "currentColor" : "none"}
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <line x1="12" y1="17" x2="12" y2="22" />
          <path d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z" />
        </svg>
      </button>
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
