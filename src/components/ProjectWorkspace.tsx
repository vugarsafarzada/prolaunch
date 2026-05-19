import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import ScriptButton from "./ScriptButton";
import LogViewer from "./LogViewer";
import type { ProjectInfo, ScriptInfo, LogEvent, ProcessEndEvent, LogLine } from "../types";

interface Props {
  project: ProjectInfo;
  onRunningChange: (projectPath: string, hasRunning: boolean) => void;
}

function ProjectWorkspace({ project, onRunningChange }: Props) {
  const [runningScripts, setRunningScripts] = useState<Set<string>>(new Set());
  const [logs, setLogs] = useState<LogLine[]>([]);
  const [activeLog, setActiveLog] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");

  const addLog = useCallback(
    (scriptName: string, text: string, isError: boolean) => {
      setLogs((prev) => [
        ...prev,
        { text, isError, timestamp: Date.now(), scriptName },
      ]);
    },
    [],
  );

  useEffect(() => {
    onRunningChange(project.path, runningScripts.size > 0);
  }, [project.path, runningScripts, onRunningChange]);

  useEffect(() => {
    let unlistenLog: UnlistenFn;
    let unlistenEnd: UnlistenFn;

    const setup = async () => {
      unlistenLog = await listen<LogEvent>("script-log", (event) => {
        const { project_path, script_name, line, is_error } = event.payload;
        if (project_path === project.path) {
          addLog(script_name, line, is_error);
        }
      });

      unlistenEnd = await listen<ProcessEndEvent>(
        "process-ended",
        (event) => {
          const { project_path, script_name, exit_code } = event.payload;
          if (project_path === project.path) {
            setRunningScripts((prev) => {
              const next = new Set(prev);
              next.delete(script_name);
              return next;
            });
            addLog(
              script_name,
              `Process exited with code ${exit_code ?? "unknown"}`,
              exit_code !== 0,
            );
          }
        },
      );
    };

    setup();

    return () => {
      unlistenLog?.();
      unlistenEnd?.();
    };
  }, [project.path, addLog]);

  const handleStart = async (script: ScriptInfo) => {
    try {
      const pid = await invoke<number>("run_script", {
        projectPath: project.path,
        scriptName: script.name,
      });
      setRunningScripts((prev) => new Set(prev).add(script.name));
      setActiveLog(script.name);
      addLog(
        script.name,
        `Starting '${script.name}' (PID: ${pid})...`,
        false,
      );
    } catch (err) {
      addLog(script.name, `Error: ${err}`, true);
    }
  };

  const handleStop = async (script: ScriptInfo) => {
    try {
      await invoke("kill_script", {
        projectPath: project.path,
        scriptName: script.name,
      });
      setRunningScripts((prev) => {
        const next = new Set(prev);
        next.delete(script.name);
        return next;
      });
      addLog(script.name, `Stopped '${script.name}'`, false);
    } catch (err) {
      addLog(script.name, `Error stopping: ${err}`, true);
    }
  };

  const runningLogs = logs.filter(
    (l) => !activeLog || l.scriptName === activeLog,
  );

  return (
    <div className="workspace">
      <div className="workspace-sidebar">
        <div className="workspace-header">
          <h2>{project.name}</h2>
          <span className="workspace-path">{project.path}</span>
        </div>
        <div className="search-box">
          <svg className="search-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="11" cy="11" r="8" />
            <line x1="21" y1="21" x2="16.65" y2="16.65" />
          </svg>
          <input
            type="text"
            className="search-input"
            placeholder="Search scripts..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
        </div>
        <div className="scripts-list">
          {project.scripts.length === 0 ? (
            <div className="no-scripts">No scripts found in package.json</div>
          ) : (
            [...project.scripts]
              .filter(
                (s) =>
                  s.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
                  s.command.toLowerCase().includes(searchQuery.toLowerCase()),
              )
              .sort((a, b) => {
                const aRunning = runningScripts.has(a.name) ? 0 : 1;
                const bRunning = runningScripts.has(b.name) ? 0 : 1;
                return aRunning - bRunning;
              })
              .map((script) => (
              <ScriptButton
                key={script.name}
                script={script}
                isRunning={runningScripts.has(script.name)}
                onStart={handleStart}
                onStop={handleStop}
              />
            ))
          )}
        </div>
      </div>
      <div className="workspace-main">
        {runningScripts.size > 0 && (
          <div className="log-tabs">
            {Array.from(runningScripts).map((name) => (
              <button
                key={name}
                className={`log-tab ${activeLog === name ? "active" : ""}`}
                onClick={() => setActiveLog(name)}
              >
                <span className="log-tab-dot" />
                {name}
              </button>
            ))}
            <button
              className={`log-tab ${activeLog === null ? "active" : ""}`}
              onClick={() => setActiveLog(null)}
            >
              All
            </button>
          </div>
        )}
        <LogViewer logs={runningLogs} onClear={() => setLogs([])} />
      </div>
    </div>
  );
}

export default ProjectWorkspace;
