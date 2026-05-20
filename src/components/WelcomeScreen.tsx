import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import CreateProjectFlow from "./CreateProjectFlow";
import type { ProjectInfo } from "../types";

interface Props {
  onProjectOpen: (project: ProjectInfo) => void;
}

function shortenPath(path: string): string {
  const parts = path.split(/[/\\]/).filter(Boolean);
  if (parts.length <= 3) return path;
  return ".../" + parts.slice(-3).join("/");
}

function WelcomeScreen({ onProjectOpen }: Props) {
  const [recentProjects, setRecentProjects] = useState<string[]>([]);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [isCreatingProject, setIsCreatingProject] = useState(false);

  useEffect(() => {
    invoke<string[]>("load_recent_projects").then(
      (paths) => setRecentProjects(paths),
      () => { },
    );
  }, []);

  const handleOpenProject = async () => {
    setErrorMessage(null);

    try {
      const folder = await open({
        directory: true,
        multiple: false,
        title: "Select Project Folder",
      });

      if (folder) {
        const project = await invoke<ProjectInfo>("read_package_json", {
          projectPath: folder,
        });
        onProjectOpen(project);
      }
    } catch (err) {
      console.error("Failed to open project:", err);
      setErrorMessage(String(err));
    }
  };

  const handleRecentOpen = async (path: string) => {
    setErrorMessage(null);

    try {
      const project = await invoke<ProjectInfo>("read_package_json", {
        projectPath: path,
      });
      onProjectOpen(project);
    } catch (err) {
      setRecentProjects((prev) => prev.filter((p) => p !== path));
      setErrorMessage(`Could not open recent project: ${String(err)}`);
    }
  };

  const handleRemoveRecent = async (path: string) => {
    setErrorMessage(null);
    setRecentProjects((prev) => prev.filter((p) => p !== path));

    try {
      await invoke("remove_recent_project", { projectPath: path });
    } catch (err) {
      setErrorMessage(`Could not remove recent project: ${String(err)}`);
      invoke<string[]>("load_recent_projects").then(
        (paths) => setRecentProjects(paths),
        () => { },
      );
    }
  };

  const handleClearRecent = async () => {
    setErrorMessage(null);
    const previousProjects = recentProjects;
    setRecentProjects([]);

    try {
      await invoke("clear_recent_projects");
    } catch (err) {
      setRecentProjects(previousProjects);
      setErrorMessage(`Could not clear recent projects: ${String(err)}`);
    }
  };

  if (isCreatingProject) {
    return (
      <div className="welcome-screen">
        <div className="welcome-content create-mode">
          <CreateProjectFlow
            onBack={() => setIsCreatingProject(false)}
            onProjectOpen={onProjectOpen}
          />
        </div>
      </div>
    );
  }

  return (
    <div className="welcome-screen">
      <div className="welcome-content">
        <div className="welcome-logo">
          <svg width="150" height="150" viewBox="0 0 150 150" fill="none" xmlns="http://www.w3.org/2000/svg">
            <rect width="150" height="150" rx="33" fill="#0345FC" />
            <rect x="38" y="38" width="75" height="75" rx="12" fill="white" />
          </svg>
        </div>
        <h1>ProLaunch</h1>
        <p className="welcome-subtitle">Developed by <strong>Vugar Safarzada</strong> <br /> (github.com/vugarsafarzada)</p>

        <div className="welcome-actions">
          <button className="welcome-btn primary" onClick={handleOpenProject}>
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
            </svg>
            Open Existing Project
          </button>

          <button className="welcome-btn secondary" onClick={() => setIsCreatingProject(true)}>
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <line x1="12" y1="5" x2="12" y2="19" />
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
            Create New Project
          </button>
        </div>

        {errorMessage && (
          <div className="welcome-error" role="alert">
            {errorMessage}
          </div>
        )}

        {recentProjects.length > 0 && (
          <div className="recent-section">
            <div className="recent-header">
              <span className="recent-label">Recent projects</span>
              <button className="recent-clear" onClick={handleClearRecent} type="button">
                Clear all
              </button>
            </div>
            <div className="recent-list">
              {recentProjects.map((path) => (
                <div className="recent-row" key={path}>
                  <button
                    className="recent-item"
                    onClick={() => handleRecentOpen(path)}
                    title={path}
                    type="button"
                  >
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
                    </svg>
                    <span className="recent-path">{shortenPath(path)}</span>
                  </button>
                  <button
                    className="recent-delete"
                    onClick={() => handleRemoveRecent(path)}
                    title="Remove from recent"
                    type="button"
                    aria-label={`Remove ${path} from recent projects`}
                  >
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <path d="M3 6h18" />
                      <path d="M8 6V4h8v2" />
                      <path d="M19 6l-1 14H6L5 6" />
                      <path d="M10 11v5" />
                      <path d="M14 11v5" />
                    </svg>
                  </button>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default WelcomeScreen;
