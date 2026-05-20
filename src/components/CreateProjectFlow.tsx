import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { ProjectInfo } from "../types";
import {
  PROJECT_TEMPLATES,
  TEMPLATE_ICONS,
  templateIconKey,
  type LanguageFilter,
  type ProjectTemplate,
} from "./templates";

type CreateStep = "gallery" | "details" | "creating";

interface Props {
  onBack: () => void;
  onProjectOpen: (project: ProjectInfo) => void;
}

interface CreateProjectLogEvent {
  creation_id: string;
  line: string;
  is_error: boolean;
}

interface CreateLogLine {
  text: string;
  isError: boolean;
}

function createLogTone(log: CreateLogLine) {
  const text = log.text.toLowerCase();

  if (
    text.startsWith("error:") ||
    text.includes(" command not found") ||
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
    text.includes("warning") ||
    text.includes("warn") ||
    text.includes("deprecated") ||
    log.isError
  ) {
    return "warning";
  }

  if (
    text.includes("success") ||
    text.includes("successfully") ||
    text.includes("completed") ||
    text.includes("requirements ready")
  ) {
    return "success";
  }

  return "default";
}

const PROJECT_NAME_PATTERN = /^[a-z0-9][a-z0-9._-]*$/;
const LANGUAGE_TABS: LanguageFilter[] = [
  "All",
  "Dart",
  "Go",
  "Java",
  "JavaScript",
  "PHP",
  "Python",
  "Ruby",
  "TypeScript",
];

function shortenPath(path: string): string {
  const parts = path.split(/[/\\]/).filter(Boolean);
  if (parts.length <= 4) return path;
  return ".../" + parts.slice(-4).join("/");
}

function targetPath(parentDir: string, projectName: string): string {
  if (!parentDir || !projectName) return "";
  const separator = parentDir.includes("\\") && !parentDir.includes("/") ? "\\" : "/";
  return `${parentDir.replace(/[\\/]+$/, "")}${separator}${projectName}`;
}

function projectNameError(projectName: string): string | null {
  if (!projectName.trim()) return "Project name is required.";
  if (!PROJECT_NAME_PATTERN.test(projectName)) {
    return "Use lowercase letters, numbers, dots, dashes, or underscores.";
  }
  return null;
}

function TemplateIcon({ template, size = "md" }: { template: ProjectTemplate; size?: "sm" | "md" }) {
  const icon = TEMPLATE_ICONS[templateIconKey(template)];

  return (
    <div
      className={`template-icon ${icon.className} ${icon.src ? "asset" : ""} ${size === "sm" ? "small" : ""}`}
      title={icon.title}
      aria-label={icon.title}
    >
      {icon.src ? (
        <img src={icon.src} alt="" aria-hidden="true" />
      ) : (
        <span>{icon.label}</span>
      )}
    </div>
  );
}

function waitForNextPaint(): Promise<void> {
  return new Promise((resolve) => {
    requestAnimationFrame(() => {
      requestAnimationFrame(() => resolve());
    });
  });
}

function CreateProjectFlow({ onBack, onProjectOpen }: Props) {
  const [step, setStep] = useState<CreateStep>("gallery");
  const [language, setLanguage] = useState<LanguageFilter>("All");
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedTemplateId, setSelectedTemplateId] = useState(PROJECT_TEMPLATES[0].versions[0].id);
  const [parentDir, setParentDir] = useState("");
  const [projectName, setProjectName] = useState("");
  const [isCreating, setIsCreating] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [createLogs, setCreateLogs] = useState<CreateLogLine[]>([]);
  const activeCreationIdRef = useRef<string | null>(null);
  const createLogBottomRef = useRef<HTMLDivElement>(null);

  const filteredTemplates = useMemo(() => {
    const normalizedQuery = searchQuery.trim().toLowerCase();

    return PROJECT_TEMPLATES.filter((template) => {
      if (language !== "All" && template.language !== language) return false;
      if (!normalizedQuery) return true;

      const searchable = [
        template.title,
        template.framework,
        template.language,
        template.description,
        ...template.tags,
        ...template.versions.flatMap((version) => [version.label, version.command]),
      ].join(" ").toLowerCase();

      return searchable.includes(normalizedQuery);
    }).sort((a, b) => {
      const languageOrder = a.language.localeCompare(b.language);
      if (languageOrder !== 0) return languageOrder;

      const titleOrder = a.title.localeCompare(b.title);
      if (titleOrder !== 0) return titleOrder;

      return a.cardId.localeCompare(b.cardId);
    });
  }, [language, searchQuery]);

  const selectedTemplate = PROJECT_TEMPLATES.find((template) =>
    template.versions.some((version) => version.id === selectedTemplateId),
  ) ?? PROJECT_TEMPLATES[0];

  const selectedVersion = selectedTemplate.versions.find((version) =>
    version.id === selectedTemplateId,
  ) ?? selectedTemplate.versions[0];

  const nameError = projectNameError(projectName);
  const canCreate = Boolean(parentDir && selectedVersion && !nameError && !isCreating);
  const previewPath = targetPath(parentDir, projectName);

  useEffect(() => {
    let isMounted = true;
    let unlisten: UnlistenFn | undefined;

    listen<CreateProjectLogEvent>("project-create-log", (event) => {
      if (event.payload.creation_id !== activeCreationIdRef.current) return;

      setCreateLogs((prev) => [
        ...prev,
        { text: event.payload.line, isError: event.payload.is_error },
      ].slice(-1000));
    }).then((fn) => {
      if (isMounted) {
        unlisten = fn;
      } else {
        fn();
      }
    });

    return () => {
      isMounted = false;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    createLogBottomRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [createLogs]);

  const handleTemplateSelect = (template: ProjectTemplate) => {
    setSelectedTemplateId((current) => {
      const stillOnTemplate = template.versions.some((version) => version.id === current);
      return stillOnTemplate ? current : template.versions[0].id;
    });
  };

  const handleChooseFolder = async () => {
    setErrorMessage(null);

    try {
      const folder = await open({
        directory: true,
        multiple: false,
        title: "Select Parent Folder",
      });

      if (typeof folder === "string") {
        setParentDir(folder);
      }
    } catch (err) {
      setErrorMessage(String(err));
    }
  };

  const handleCreate = async () => {
    if (!canCreate) return;

    const creationId = crypto.randomUUID();
    activeCreationIdRef.current = creationId;
    setStep("creating");
    setIsCreating(true);
    setErrorMessage(null);
    setCreateLogs([
      { text: `Preparing ${selectedTemplate.title} project...`, isError: false },
      { text: `Target: ${previewPath}`, isError: false },
    ]);

    try {
      await waitForNextPaint();
      const project = await invoke<ProjectInfo>("create_project", {
        templateId: selectedVersion.id,
        parentDir,
        projectName,
        creationId,
      });
      activeCreationIdRef.current = null;
      onProjectOpen(project);
    } catch (err) {
      activeCreationIdRef.current = null;
      setErrorMessage(String(err));
      setCreateLogs((prev) => [
        ...prev,
        { text: `Error: ${String(err)}`, isError: true },
      ].slice(-1000));
    } finally {
      setIsCreating(false);
    }
  };

  return (
    <div className="create-flow">
      <div className="create-topbar">
        <button
          className="create-back"
          onClick={step === "gallery" ? onBack : step === "creating" ? () => setStep("details") : () => setStep("gallery")}
          disabled={isCreating}
          type="button"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="m15 18-6-6 6-6" />
          </svg>
          Back
        </button>
        <div>
          <h2>Create Project</h2>
          <span>
            {step === "gallery"
              ? "Choose a starter"
              : step === "details"
                ? "Configure project"
                : isCreating
                  ? "Installing project"
                  : "Create failed"}
          </span>
        </div>
      </div>

      {step === "gallery" ? (
        <>
          <div className="template-toolbar">
            <div className="template-tabs">
              {LANGUAGE_TABS.map((item) => (
                <button
                  key={item}
                  className={`template-tab ${language === item ? "active" : ""}`}
                  onClick={() => setLanguage(item)}
                  type="button"
                >
                  {item}
                </button>
              ))}
            </div>
            <div className="template-search">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="11" cy="11" r="8" />
                <line x1="21" y1="21" x2="16.65" y2="16.65" />
              </svg>
              <input
                value={searchQuery}
                onChange={(event) => setSearchQuery(event.target.value)}
                placeholder="Search templates..."
              />
            </div>
          </div>

          <div className="template-grid">
            {filteredTemplates.map((template) => {
              const isSelected = template.versions.some((version) => version.id === selectedTemplateId);
              const activeVersion = isSelected
                ? selectedVersion
                : template.versions[0];

              return (
                <div
                  key={template.cardId}
                  className={`template-card ${isSelected ? "selected" : ""}`}
                  onClick={() => handleTemplateSelect(template)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      handleTemplateSelect(template);
                    }
                  }}
                  role="button"
                  tabIndex={0}
                >
                  <div className="template-card-header">
                    <div className="template-title-group">
                      <TemplateIcon template={template} />
                      <div>
                        <h3>{template.title}</h3>
                        <span>{template.language}</span>
                      </div>
                    </div>
                    <span className="template-framework">{template.framework}</span>
                  </div>
                  <p>{template.description}</p>
                  <div className="template-tags">
                    {template.tags.map((tag) => (
                      <span key={tag}>{tag}</span>
                    ))}
                  </div>
                  <div className="template-version-row">
                    <label>Version</label>
                    <select
                      value={activeVersion.id}
                      onClick={(event) => event.stopPropagation()}
                      onChange={(event) => setSelectedTemplateId(event.target.value)}
                    >
                      {template.versions.map((version) => (
                        <option key={version.id} value={version.id}>
                          {version.label}
                        </option>
                      ))}
                    </select>
                  </div>
                  <code>{activeVersion.command}</code>
                </div>
              );
            })}
          </div>

          {filteredTemplates.length === 0 && (
            <div className="template-empty">No templates found.</div>
          )}

          <div className="create-footer">
            <span>{selectedTemplate.title} / {selectedTemplate.language} / {selectedVersion.label}</span>
            <button
              className="create-primary"
              onClick={() => setStep("details")}
              disabled={filteredTemplates.length === 0}
              type="button"
            >
              Next
            </button>
          </div>
        </>
      ) : step === "details" ? (
        <div className="create-details">
          <div className="selected-template-summary">
            <span className="summary-label">Selected</span>
            <div className="summary-title-row">
              <TemplateIcon template={selectedTemplate} size="sm" />
              <strong>{selectedTemplate.title}</strong>
            </div>
            <span>{selectedTemplate.language} / {selectedVersion.label}</span>
            <code>{selectedVersion.command}</code>
          </div>

          <div className="create-field">
            <label>Parent folder</label>
            <div className="folder-picker-row">
              <button className="create-secondary" onClick={handleChooseFolder} type="button">
                Choose Folder
              </button>
              <span title={parentDir}>{parentDir ? shortenPath(parentDir) : "No folder selected"}</span>
            </div>
          </div>

          <div className="create-field">
            <label>Project name</label>
            <input
              value={projectName}
              onChange={(event) => setProjectName(event.target.value)}
              placeholder="my-app"
              spellCheck={false}
            />
            {projectName && nameError && <span className="field-error">{nameError}</span>}
          </div>

          {previewPath && (
            <div className="target-preview">
              <span>Target</span>
              <code>{previewPath}</code>
            </div>
          )}

          {errorMessage && (
            <div className="welcome-error" role="alert">
              {errorMessage}
            </div>
          )}

          <div className="create-footer">
            <button className="create-secondary" onClick={() => setStep("gallery")} type="button">
              Back
            </button>
            <button
              className="create-primary"
              disabled={!canCreate}
              onClick={handleCreate}
              type="button"
            >
              {isCreating ? "Creating..." : "Create"}
            </button>
          </div>
        </div>
      ) : (
        <div className="create-progress">
          <div className={`create-progress-hero ${isCreating ? "" : "failed"}`}>
            {isCreating ? (
              <div className="create-spinner" />
            ) : (
              <div className="create-status-icon error" aria-hidden="true">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M18 6 6 18" />
                  <path d="m6 6 12 12" />
                </svg>
              </div>
            )}
            <div>
              <strong>{isCreating ? `Creating ${projectName}` : "Create failed"}</strong>
              <span>
                {isCreating
                  ? "Installing dependencies and preparing the workspace..."
                  : "Review the logs below, adjust settings, and try again."}
              </span>
            </div>
          </div>

          <div className="selected-template-summary">
            <span className="summary-label">Selected</span>
            <div className="summary-title-row">
              <TemplateIcon template={selectedTemplate} size="sm" />
              <strong>{selectedTemplate.title}</strong>
            </div>
            <span>{selectedTemplate.language} / {selectedVersion.label}</span>
            <code>{selectedVersion.command}</code>
          </div>

          <div className="target-preview">
            <span>Target</span>
            <code>{previewPath}</code>
          </div>

          <div className="create-log-panel">
            <div className="create-log-header">
              <span>Install logs</span>
              <span>{isCreating ? "Live output" : "Stopped"}</span>
            </div>
            <div className="create-log-content">
              {createLogs.map((log, index) => (
                <div key={`${index}-${log.text}`} className={`create-log-line ${createLogTone(log)}`}>
                  <span>{String(index + 1).padStart(2, "0")}</span>
                  <code>{log.text}</code>
                </div>
              ))}
              <div ref={createLogBottomRef} />
            </div>
          </div>

          {errorMessage && (
            <div className="welcome-error" role="alert">
              {errorMessage}
            </div>
          )}

          <div className="create-footer">
            <button
              className="create-secondary"
              disabled={isCreating}
              onClick={() => setStep("details")}
              type="button"
            >
              Back to settings
            </button>
            <button
              className="create-primary"
              disabled={isCreating}
              onClick={handleCreate}
              type="button"
            >
              {isCreating ? "Creating..." : "Try again"}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

export default CreateProjectFlow;
