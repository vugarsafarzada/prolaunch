import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
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
  const [pinnedScripts, setPinnedScripts] = useState<Set<string>>(new Set());
  const [showFolderMenu, setShowFolderMenu] = useState(false);
  const folderRef = useRef<HTMLDivElement>(null);
  const startingScriptsRef = useRef<Set<string>>(new Set());

  const isMac = navigator.platform.toUpperCase().indexOf("MAC") >= 0;
  const isWindows = navigator.platform.toUpperCase().indexOf("WIN") >= 0;
  const folderLabel = isMac ? "Finder" : isWindows ? "Explorer" : "File Manager";

  useEffect(() => {
    const handleClick = (e: MouseEvent) => {
      if (folderRef.current && !folderRef.current.contains(e.target as Node)) {
        setShowFolderMenu(false);
      }
    };
    if (showFolderMenu) {
      document.addEventListener("mousedown", handleClick);
    }
    return () => document.removeEventListener("mousedown", handleClick);
  }, [showFolderMenu]);

  const handleOpenFolder = () => revealItemInDir(project.path);
  const handleOpenVSCode = () => invoke("open_in_vscode", { path: project.path });
  const handleOpenTerminal = () => invoke("open_in_terminal", { path: project.path });

  const addLog = useCallback(
    (scriptName: string, text: string, isError: boolean) => {
      setLogs((prev) => [
        ...prev,
        { text, isError, timestamp: Date.now(), scriptName },
      ]);
    },
    [],
  );

  const setScriptRunning = useCallback((scriptName: string, isRunning: boolean) => {
    setRunningScripts((prev) => {
      if (isRunning === prev.has(scriptName)) {
        return prev;
      }

      const next = new Set(prev);
      if (isRunning) {
        next.add(scriptName);
      } else {
        next.delete(scriptName);
      }
      return next;
    });
  }, []);

  const pinsLoaded = useRef(false);

  useEffect(() => {
    pinsLoaded.current = false;
    invoke<string[]>("load_pins", { projectPath: project.path }).then(
      (pins) => {
        setPinnedScripts(new Set(pins));
        pinsLoaded.current = true;
      },
      () => {},
    );
  }, [project.path]);

  useEffect(() => {
    if (!pinsLoaded.current) return;
    invoke("save_pins", {
      projectPath: project.path,
      pins: Array.from(pinnedScripts),
    }).catch(() => {});
  }, [project.path, pinnedScripts]);

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
            setScriptRunning(script_name, false);
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
  }, [project.path, addLog, setScriptRunning]);

  const handleStart = async (script: ScriptInfo) => {
    if (runningScripts.has(script.name) || startingScriptsRef.current.has(script.name)) {
      return;
    }

    startingScriptsRef.current.add(script.name);
    setScriptRunning(script.name, true);
    setActiveLog(script.name);
    addLog(script.name, `Starting '${script.name}'...`, false);

    try {
      const pid = await invoke<number>("run_script", {
        projectPath: project.path,
        scriptName: script.name,
        packageManager: project.packageManager,
      });
      addLog(script.name, `Started with ${project.packageManager} (PID: ${pid})`, false);
    } catch (err) {
      setScriptRunning(script.name, false);
      addLog(script.name, `Error: ${err}`, true);
    } finally {
      startingScriptsRef.current.delete(script.name);
    }
  };

  const handleTogglePin = (script: ScriptInfo) => {
    setPinnedScripts((prev) => {
      const next = new Set(prev);
      if (next.has(script.name)) {
        next.delete(script.name);
      } else {
        next.add(script.name);
      }
      return next;
    });
  };

  const handleRestart = async (script: ScriptInfo) => {
    let stopped = false;

    try {
      await invoke("kill_script", {
        projectPath: project.path,
        scriptName: script.name,
      });
      stopped = true;
      setScriptRunning(script.name, false);
      addLog(script.name, `Restarting '${script.name}'...`, false);
      setScriptRunning(script.name, true);
      const pid = await invoke<number>("run_script", {
        projectPath: project.path,
        scriptName: script.name,
        packageManager: project.packageManager,
      });
      setActiveLog(script.name);
      addLog(script.name, `Restarted with ${project.packageManager} (PID: ${pid})`, false);
    } catch (err) {
      if (stopped) {
        setScriptRunning(script.name, false);
      }
      addLog(script.name, `Error restarting: ${err}`, true);
    }
  };

  const handleStop = async (script: ScriptInfo) => {
    try {
      await invoke("kill_script", {
        projectPath: project.path,
        scriptName: script.name,
      });
      setScriptRunning(script.name, false);
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
          <div className="workspace-title-row">
            <h2>{project.name}</h2>
            <div className="folder-actions" ref={folderRef}>
              <button
                className="btn-folder"
                onClick={() => setShowFolderMenu((p) => !p)}
                title="Open project location"
              >
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
                </svg>
              </button>
              {showFolderMenu && (
                <div className="folder-dropdown">
                  <button onClick={handleOpenFolder}>{folderLabel}</button>
                  <button onClick={handleOpenVSCode}>VS Code</button>
                  <button onClick={handleOpenTerminal}>Terminal</button>
                </div>
              )}
            </div>
          </div>
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
            <div className="no-scripts">No scripts found in project manifest</div>
          ) : (
            [...project.scripts]
              .filter(
                (s) =>
                  s.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
                  s.command.toLowerCase().includes(searchQuery.toLowerCase()),
              )
              .sort((a, b) => {
                const aPinned = pinnedScripts.has(a.name) ? 0 : 1;
                const bPinned = pinnedScripts.has(b.name) ? 0 : 1;
                if (aPinned !== bPinned) return aPinned - bPinned;
                const aRunning = runningScripts.has(a.name) ? 0 : 1;
                const bRunning = runningScripts.has(b.name) ? 0 : 1;
                if (aRunning !== bRunning) return aRunning - bRunning;
                return a.name.localeCompare(b.name);
              })
              .map((script) => (
              <ScriptButton
                key={script.name}
                script={script}
                isRunning={runningScripts.has(script.name)}
                isPinned={pinnedScripts.has(script.name)}
                onStart={handleStart}
                onStop={handleStop}
                onTogglePin={handleTogglePin}
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
        <LogViewer
          logs={runningLogs}
          activeScript={activeLog}
          isRunning={activeLog ? runningScripts.has(activeLog) : false}
          onClear={() => setLogs([])}
          onReRun={() => {
            const sc = activeLog ? project.scripts.find((s) => s.name === activeLog) : null;
            if (sc) handleRestart(sc);
          }}
        />
      </div>
    </div>
  );
}

export default ProjectWorkspace;
