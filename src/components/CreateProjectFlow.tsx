import { useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { ProjectInfo } from "../types";

type LanguageFilter = "All" | "JavaScript" | "TypeScript";
type CreateStep = "gallery" | "details";

interface TemplateVersion {
  id: string;
  label: string;
  command: string;
}

interface ProjectTemplate {
  cardId: string;
  title: string;
  framework: string;
  language: Exclude<LanguageFilter, "All">;
  description: string;
  tags: string[];
  versions: TemplateVersion[];
}

interface Props {
  onBack: () => void;
  onProjectOpen: (project: ProjectInfo) => void;
}

const PROJECT_NAME_PATTERN = /^[a-z0-9][a-z0-9._-]*$/;

const PROJECT_TEMPLATES: ProjectTemplate[] = [
  {
    cardId: "vite-react-ts",
    title: "React + Vite",
    framework: "React",
    language: "TypeScript",
    description: "Fast React app powered by Vite.",
    tags: ["Vite", "React", "SPA"],
    versions: [
      {
        id: "vite-react-ts",
        label: "Latest",
        command: "npx create-vite@latest my-app --template react-ts",
      },
    ],
  },
  {
    cardId: "vite-react-js",
    title: "React + Vite",
    framework: "React",
    language: "JavaScript",
    description: "Fast React app powered by Vite.",
    tags: ["Vite", "React", "SPA"],
    versions: [
      {
        id: "vite-react-js",
        label: "Latest",
        command: "npx create-vite@latest my-app --template react",
      },
    ],
  },
  {
    cardId: "next-ts",
    title: "Next.js",
    framework: "Next.js",
    language: "TypeScript",
    description: "Full-stack React app with App Router.",
    tags: ["React", "SSR", "App Router"],
    versions: [
      { id: "next-ts-latest", label: "Latest", command: "npx create-next-app@latest my-app --ts" },
      { id: "next-ts-16", label: "16", command: "npx create-next-app@16 my-app --ts" },
      { id: "next-ts-15", label: "15", command: "npx create-next-app@15 my-app --ts" },
      { id: "next-ts-14", label: "14", command: "npx create-next-app@14 my-app --ts" },
    ],
  },
  {
    cardId: "next-js",
    title: "Next.js",
    framework: "Next.js",
    language: "JavaScript",
    description: "Full-stack React app with App Router.",
    tags: ["React", "SSR", "App Router"],
    versions: [
      { id: "next-js-latest", label: "Latest", command: "npx create-next-app@latest my-app --js" },
      { id: "next-js-16", label: "16", command: "npx create-next-app@16 my-app --js" },
      { id: "next-js-15", label: "15", command: "npx create-next-app@15 my-app --js" },
      { id: "next-js-14", label: "14", command: "npx create-next-app@14 my-app --js" },
    ],
  },
  {
    cardId: "cra-ts",
    title: "Create React App",
    framework: "React",
    language: "TypeScript",
    description: "Classic React starter with react-scripts.",
    tags: ["React", "CRA"],
    versions: [
      {
        id: "cra-ts",
        label: "Latest",
        command: "npx create-react-app@latest my-app --template typescript",
      },
    ],
  },
  {
    cardId: "cra-js",
    title: "Create React App",
    framework: "React",
    language: "JavaScript",
    description: "Classic React starter with react-scripts.",
    tags: ["React", "CRA"],
    versions: [
      {
        id: "cra-js",
        label: "Latest",
        command: "npx create-react-app@latest my-app",
      },
    ],
  },
  {
    cardId: "vite-vue-ts",
    title: "Vue + Vite",
    framework: "Vue",
    language: "TypeScript",
    description: "Vue starter generated with Vite.",
    tags: ["Vite", "Vue"],
    versions: [
      {
        id: "vite-vue-ts",
        label: "Latest",
        command: "npx create-vite@latest my-app --template vue-ts",
      },
    ],
  },
  {
    cardId: "vite-vue-js",
    title: "Vue + Vite",
    framework: "Vue",
    language: "JavaScript",
    description: "Vue starter generated with Vite.",
    tags: ["Vite", "Vue"],
    versions: [
      {
        id: "vite-vue-js",
        label: "Latest",
        command: "npx create-vite@latest my-app --template vue",
      },
    ],
  },
  {
    cardId: "vite-svelte-ts",
    title: "Svelte + Vite",
    framework: "Svelte",
    language: "TypeScript",
    description: "Svelte starter generated with Vite.",
    tags: ["Vite", "Svelte"],
    versions: [
      {
        id: "vite-svelte-ts",
        label: "Latest",
        command: "npx create-vite@latest my-app --template svelte-ts",
      },
    ],
  },
  {
    cardId: "vite-svelte-js",
    title: "Svelte + Vite",
    framework: "Svelte",
    language: "JavaScript",
    description: "Svelte starter generated with Vite.",
    tags: ["Vite", "Svelte"],
    versions: [
      {
        id: "vite-svelte-js",
        label: "Latest",
        command: "npx create-vite@latest my-app --template svelte",
      },
    ],
  },
  {
    cardId: "angular-ts",
    title: "Angular",
    framework: "Angular",
    language: "TypeScript",
    description: "Angular workspace generated with Angular CLI.",
    tags: ["Angular", "CLI"],
    versions: [
      {
        id: "angular-ts-latest",
        label: "Latest",
        command: "npx @angular/cli@latest new my-app",
      },
    ],
  },
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

function CreateProjectFlow({ onBack, onProjectOpen }: Props) {
  const [step, setStep] = useState<CreateStep>("gallery");
  const [language, setLanguage] = useState<LanguageFilter>("All");
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedTemplateId, setSelectedTemplateId] = useState(PROJECT_TEMPLATES[0].versions[0].id);
  const [parentDir, setParentDir] = useState("");
  const [projectName, setProjectName] = useState("");
  const [isCreating, setIsCreating] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

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

    setIsCreating(true);
    setErrorMessage(null);

    try {
      const project = await invoke<ProjectInfo>("create_project", {
        templateId: selectedVersion.id,
        parentDir,
        projectName,
      });
      onProjectOpen(project);
    } catch (err) {
      setErrorMessage(String(err));
    } finally {
      setIsCreating(false);
    }
  };

  return (
    <div className="create-flow">
      <div className="create-topbar">
        <button
          className="create-back"
          onClick={step === "gallery" ? onBack : () => setStep("gallery")}
          type="button"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="m15 18-6-6 6-6" />
          </svg>
          Back
        </button>
        <div>
          <h2>Create Project</h2>
          <span>{step === "gallery" ? "Choose a starter" : "Configure project"}</span>
        </div>
      </div>

      {step === "gallery" ? (
        <>
          <div className="template-toolbar">
            <div className="template-tabs">
              {(["All", "JavaScript", "TypeScript"] as LanguageFilter[]).map((item) => (
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
                    <div>
                      <h3>{template.title}</h3>
                      <span>{template.language}</span>
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
      ) : (
        <div className="create-details">
          <div className="selected-template-summary">
            <span className="summary-label">Selected</span>
            <strong>{selectedTemplate.title}</strong>
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
      )}
    </div>
  );
}

export default CreateProjectFlow;
