import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { ProjectInfo } from "../types";

interface Props {
  onProjectOpen: (project: ProjectInfo) => void;
}

function WelcomeScreen({ onProjectOpen }: Props) {
  const handleOpenProject = async () => {
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
    }
  };

  return (
    <div className="welcome-screen">
      <div className="welcome-content">
        <div className="welcome-logo">
          <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="16 18 22 12 16 6" />
            <polyline points="8 6 2 12 8 18" />
          </svg>
        </div>
        <h1>DevLauncher</h1>
        <p className="welcome-subtitle">Manage your dev scripts with ease</p>

        <div className="welcome-actions">
          <button className="welcome-btn primary" onClick={handleOpenProject}>
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
            </svg>
            Open Existing Project
          </button>

          <button className="welcome-btn secondary" onClick={() => {}}>
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <line x1="12" y1="5" x2="12" y2="19" />
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
            Create New Project
          </button>
        </div>

        <p className="welcome-hint">
          Open a folder containing a <code>package.json</code> to get started
        </p>
      </div>
    </div>
  );
}

export default WelcomeScreen;
