import { useState, useEffect, useCallback, useRef, useMemo, type FormEvent } from "react";
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

type CommandItem = ScriptInfo & { key: string; isCustom?: boolean };

function ProjectWorkspace({ project, onRunningChange }: Props) {
  const [runningScripts, setRunningScripts] = useState<Set<string>>(new Set());
  const [logs, setLogs] = useState<LogLine[]>([]);
  const [activeLog, setActiveLog] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [pinnedScripts, setPinnedScripts] = useState<Set<string>>(new Set());
  const [customCommands, setCustomCommands] = useState<ScriptInfo[]>([]);
  const [isAddingCustomCommand, setIsAddingCustomCommand] = useState(false);
  const [customCommandName, setCustomCommandName] = useState("");
  const [customCommandValue, setCustomCommandValue] = useState("");
  const [customCommandError, setCustomCommandError] = useState("");
  const [showFolderMenu, setShowFolderMenu] = useState(false);
  const folderRef = useRef<HTMLDivElement>(null);
  const startingScriptsRef = useRef<Set<string>>(new Set());

  const isMac = navigator.platform.toUpperCase().indexOf("MAC") >= 0;
  const isWindows = navigator.platform.toUpperCase().indexOf("WIN") >= 0;
  const folderLabel = isMac ? "Finder" : isWindows ? "Explorer" : "File Manager";

  const makeCommandKey = useCallback(
    (script: ScriptInfo & { isCustom?: boolean }) => {
      const manager = script.isCustom ? "custom" : script.packageManager ?? project.packageManager;
      return `${manager}:${script.source ?? "manifest"}:${script.name}`;
    },
    [project.packageManager],
  );

  const commandItems: CommandItem[] = useMemo(
    () => [
      ...project.scripts.map((script) => {
        const item = { ...script, isCustom: false };
        return { ...item, key: makeCommandKey(item) };
      }),
      ...customCommands.map((script) => {
        const item = {
          ...script,
          packageManager: script.packageManager ?? "custom",
          source: script.source ?? "Custom",
          isCustom: true,
        };
        return { ...item, key: makeCommandKey(item) };
      }),
    ],
    [project.scripts, customCommands, makeCommandKey],
  );

  const isPinnedCommand = useCallback(
    (script: CommandItem) => pinnedScripts.has(script.key) || pinnedScripts.has(script.name),
    [pinnedScripts],
  );

  const visibleCommands = useMemo(
    () =>
      commandItems
        .filter(
          (script) =>
            script.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
            script.command.toLowerCase().includes(searchQuery.toLowerCase()) ||
            (script.source ?? "").toLowerCase().includes(searchQuery.toLowerCase()),
        )
        .sort((a, b) => {
          const aPinned = isPinnedCommand(a) ? 0 : 1;
          const bPinned = isPinnedCommand(b) ? 0 : 1;
          if (aPinned !== bPinned) return aPinned - bPinned;
          const aRunning = runningScripts.has(a.key) ? 0 : 1;
          const bRunning = runningScripts.has(b.key) ? 0 : 1;
          if (aRunning !== bRunning) return aRunning - bRunning;
          return a.name.localeCompare(b.name);
        }),
    [commandItems, searchQuery, isPinnedCommand, runningScripts],
  );

  const activeCommand = activeLog
    ? commandItems.find((script) => script.key === activeLog)
    : null;

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
    let isActive = true;
    setCustomCommands([]);
    setIsAddingCustomCommand(false);
    setCustomCommandName("");
    setCustomCommandValue("");
    setCustomCommandError("");

    invoke<ScriptInfo[]>("load_custom_commands", { projectPath: project.path }).then(
      (commands) => {
        if (isActive) {
          setCustomCommands(commands);
        }
      },
      () => {},
    );

    return () => {
      isActive = false;
    };
  }, [project.path]);

  useEffect(() => {
    onRunningChange(project.path, runningScripts.size > 0);
  }, [project.path, runningScripts, onRunningChange]);

  useEffect(() => {
    let isActive = true;
    const unlisteners: UnlistenFn[] = [];

    const keepListener = (unlisten: UnlistenFn) => {
      if (isActive) {
        unlisteners.push(unlisten);
      } else {
        unlisten();
      }
    };

    const setup = async () => {
      const unlistenLog = await listen<LogEvent>("script-log", (event) => {
        const { project_path, script_name, line, is_error } = event.payload;
        if (project_path === project.path) {
          addLog(script_name, line, is_error);
        }
      });
      keepListener(unlistenLog);

      const unlistenEnd = await listen<ProcessEndEvent>(
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
      keepListener(unlistenEnd);
    };

    setup().catch(() => {});

    return () => {
      isActive = false;
      unlisteners.forEach((unlisten) => unlisten());
      unlisteners.length = 0;
    };
  }, [project.path, addLog, setScriptRunning]);

  const saveCustomCommands = async (commands: ScriptInfo[]) => {
    await invoke("save_custom_commands", {
      projectPath: project.path,
      commands,
    });
  };

  const resetCustomCommandForm = () => {
    setIsAddingCustomCommand(false);
    setCustomCommandName("");
    setCustomCommandValue("");
    setCustomCommandError("");
  };

  const handleAddCustomCommand = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const name = customCommandName.trim();
    const command = customCommandValue.trim();

    if (!name || !command) {
      setCustomCommandError("Name and command are required.");
      return;
    }

    const duplicate = customCommands.some(
      (script) => script.name.toLowerCase() === name.toLowerCase(),
    );

    if (duplicate) {
      setCustomCommandError("A custom command with this name already exists.");
      return;
    }

    const nextCommands = [
      ...customCommands,
      { name, command, packageManager: "custom", source: "Custom" },
    ];
    setCustomCommands(nextCommands);

    try {
      await saveCustomCommands(nextCommands);
      resetCustomCommandForm();
    } catch (err) {
      setCustomCommands(customCommands);
      setCustomCommandError(`Failed to save command: ${err}`);
    }
  };

  const handleRemoveCustomCommand = async (script: CommandItem) => {
    const nextCommands = customCommands.filter((command) => command.name !== script.name);
    setCustomCommands(nextCommands);
    setPinnedScripts((prev) => {
      const next = new Set(prev);
      next.delete(script.name);
      next.delete(script.key);
      return next;
    });

    try {
      await saveCustomCommands(nextCommands);
    } catch (err) {
      setCustomCommands(customCommands);
      addLog(script.key, `Error removing custom command: ${err}`, true);
    }
  };

  const handleStart = async (script: CommandItem) => {
    if (runningScripts.has(script.key) || startingScriptsRef.current.has(script.key)) {
      return;
    }

    startingScriptsRef.current.add(script.key);
    setScriptRunning(script.key, true);
    setActiveLog(script.key);
    addLog(script.key, `Starting '${script.name}'...`, false);

    try {
      const packageManager = script.isCustom ? "custom" : script.packageManager ?? project.packageManager;
      const pid = await invoke<number>("run_script", {
        projectPath: project.path,
        scriptName: script.name,
        packageManager,
        runKey: script.key,
      });
      addLog(
        script.key,
        `Started with ${script.isCustom ? "custom command" : packageManager} (PID: ${pid})`,
        false,
      );
    } catch (err) {
      setScriptRunning(script.key, false);
      addLog(script.key, `Error: ${err}`, true);
    } finally {
      startingScriptsRef.current.delete(script.key);
    }
  };

  const handleTogglePin = (script: CommandItem) => {
    setPinnedScripts((prev) => {
      const next = new Set(prev);
      next.delete(script.name);
      if (next.has(script.key)) {
        next.delete(script.key);
      } else {
        next.add(script.key);
      }
      return next;
    });
  };

  const handleRestart = async (script: CommandItem) => {
    let stopped = false;

    try {
      await invoke("kill_script", {
        projectPath: project.path,
        scriptName: script.key,
      });
      stopped = true;
      setScriptRunning(script.key, false);
      addLog(script.key, `Restarting '${script.name}'...`, false);
      setScriptRunning(script.key, true);
      const packageManager = script.isCustom ? "custom" : script.packageManager ?? project.packageManager;
      const pid = await invoke<number>("run_script", {
        projectPath: project.path,
        scriptName: script.name,
        packageManager,
        runKey: script.key,
      });
      setActiveLog(script.key);
      addLog(
        script.key,
        `Restarted with ${script.isCustom ? "custom command" : packageManager} (PID: ${pid})`,
        false,
      );
    } catch (err) {
      if (stopped) {
        setScriptRunning(script.key, false);
      }
      addLog(script.key, `Error restarting: ${err}`, true);
    }
  };

  const handleStop = async (script: CommandItem) => {
    try {
      await invoke("kill_script", {
        projectPath: project.path,
        scriptName: script.key,
      });
      setScriptRunning(script.key, false);
      addLog(script.key, `Stopped '${script.name}'`, false);
    } catch (err) {
      addLog(script.key, `Error stopping: ${err}`, true);
    }
  };

  const runningLogs = logs.filter(
    (log) => !activeLog || log.scriptName === activeLog,
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
                onClick={() => setShowFolderMenu((prev) => !prev)}
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
            placeholder="Search commands..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
        </div>
        <div className="scripts-list">
          <button
            type="button"
            className="script-card add-command-card"
            onClick={() => setIsAddingCustomCommand(true)}
          >
            <span className="add-command-icon">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M12 5v14" />
                <path d="M5 12h14" />
              </svg>
            </span>
            <span className="script-info">
              <span className="script-name">Add custom command</span>
              <span className="script-command">Run a saved command in this project</span>
            </span>
          </button>

          {isAddingCustomCommand && (
            <form className="custom-command-form" onSubmit={handleAddCustomCommand}>
              <input
                className="custom-command-input"
                value={customCommandName}
                onChange={(e) => {
                  setCustomCommandName(e.target.value);
                  setCustomCommandError("");
                }}
                placeholder="Name"
                autoFocus
              />
              <input
                className="custom-command-input command"
                value={customCommandValue}
                onChange={(e) => {
                  setCustomCommandValue(e.target.value);
                  setCustomCommandError("");
                }}
                placeholder="Command, e.g. python main.py"
              />
              {customCommandError && (
                <div className="custom-command-error">{customCommandError}</div>
              )}
              <div className="custom-command-actions">
                <button type="button" className="btn-custom-cancel" onClick={resetCustomCommandForm}>
                  Cancel
                </button>
                <button type="submit" className="btn-custom-save">
                  Save
                </button>
              </div>
            </form>
          )}

          {visibleCommands.length === 0 ? (
            <div className="no-scripts">No commands found</div>
          ) : (
            visibleCommands.map((script) => (
              <ScriptButton
                key={script.key}
                script={script}
                isRunning={runningScripts.has(script.key)}
                isPinned={isPinnedCommand(script)}
                onStart={handleStart}
                onStop={handleStop}
                onTogglePin={handleTogglePin}
                onRemove={script.isCustom ? handleRemoveCustomCommand : undefined}
              />
            ))
          )}
        </div>
      </div>
      <div className="workspace-main">
        {runningScripts.size > 0 && (
          <div className="log-tabs">
            {Array.from(runningScripts).map((key) => {
              const command = commandItems.find((script) => script.key === key);
              return (
                <button
                  key={key}
                  className={`log-tab ${activeLog === key ? "active" : ""}`}
                  onClick={() => setActiveLog(key)}
                >
                  <span className="log-tab-dot" />
                  {command?.name ?? key}
                </button>
              );
            })}
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
          activeScript={activeCommand?.name ?? activeLog}
          isRunning={activeLog ? runningScripts.has(activeLog) : false}
          onClear={() => setLogs([])}
          onReRun={() => {
            if (activeCommand) handleRestart(activeCommand);
          }}
        />
      </div>
    </div>
  );
}

export default ProjectWorkspace;
