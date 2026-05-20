import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { ProjectInfo } from "../types";
import angularIcon from "../../src-tauri/icons/supports/angular_logo.png";
import codeigniterIcon from "../../src-tauri/icons/supports/codeIgniter_logo.png";
import dartIcon from "../../src-tauri/icons/supports/dart_logo.png";
import djangoIcon from "../../src-tauri/icons/supports/django_logo.png";
import expressIcon from "../../src-tauri/icons/supports/express_logo.png";
import fastapiIcon from "../../src-tauri/icons/supports/fastapi_logo.png";
import flaskIcon from "../../src-tauri/icons/supports/flask_logo.png";
import flutterIcon from "../../src-tauri/icons/supports/flutter_logo.png";
import javascriptIcon from "../../src-tauri/icons/supports/javascript_logo.png";
import laravelIcon from "../../src-tauri/icons/supports/laravel_logo.png";
import nestjsIcon from "../../src-tauri/icons/supports/nestjs_logo.png";
import nextIcon from "../../src-tauri/icons/supports/nextjs_logo.png";
import nodeIcon from "../../src-tauri/icons/supports/nodejs_logo.png";
import nuxtIcon from "../../src-tauri/icons/supports/nuxtjs_logo.png";
import phpIcon from "../../src-tauri/icons/supports/php_logo.png";
import pythonIcon from "../../src-tauri/icons/supports/python_logo.png";
import reactIcon from "../../src-tauri/icons/supports/react_logo.png";
import slimIcon from "../../src-tauri/icons/supports/slim_logo.png";
import svelteIcon from "../../src-tauri/icons/supports/svelte_logo.png";
import symfonyIcon from "../../src-tauri/icons/supports/symfony_logo.png";
import typescriptIcon from "../../src-tauri/icons/supports/typescript_logo.png";
import vueIcon from "../../src-tauri/icons/supports/vuejs_logo.png";

type LanguageFilter = "All" | "JavaScript" | "TypeScript" | "Python" | "PHP" | "Dart";
type CreateStep = "gallery" | "details" | "creating";

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

interface CreateProjectLogEvent {
  creation_id: string;
  line: string;
  is_error: boolean;
}

interface CreateLogLine {
  text: string;
  isError: boolean;
}

type TemplateIconKey =
  | "angular"
  | "codeigniter"
  | "dart"
  | "django"
  | "express"
  | "fastapi"
  | "flutter"
  | "flask"
  | "javascript"
  | "laravel"
  | "nestjs"
  | "next"
  | "node"
  | "nuxt"
  | "php"
  | "python"
  | "react"
  | "slim"
  | "svelte"
  | "symfony"
  | "typescript"
  | "vue";

interface TemplateIconMeta {
  label: string;
  title: string;
  className: string;
  src?: string;
}

const PROJECT_NAME_PATTERN = /^[a-z0-9][a-z0-9._-]*$/;

const TEMPLATE_ICONS: Record<TemplateIconKey, TemplateIconMeta> = {
  angular: { label: "A", title: "Angular", className: "angular", src: angularIcon },
  codeigniter: { label: "CI", title: "CodeIgniter", className: "codeigniter", src: codeigniterIcon },
  dart: { label: "D", title: "Dart", className: "dart", src: dartIcon },
  django: { label: "Dj", title: "Django", className: "django", src: djangoIcon },
  express: { label: "Ex", title: "Express", className: "express", src: expressIcon },
  fastapi: { label: "FA", title: "FastAPI", className: "fastapi", src: fastapiIcon },
  flutter: { label: "Fl", title: "Flutter", className: "flutter", src: flutterIcon },
  flask: { label: "Fl", title: "Flask", className: "flask", src: flaskIcon },
  javascript: { label: "JS", title: "JavaScript", className: "javascript", src: javascriptIcon },
  laravel: { label: "L", title: "Laravel", className: "laravel", src: laravelIcon },
  nestjs: { label: "Ne", title: "NestJS", className: "nestjs", src: nestjsIcon },
  next: { label: "N", title: "Next.js", className: "next", src: nextIcon },
  node: { label: "N", title: "Node.js", className: "node", src: nodeIcon },
  nuxt: { label: "N", title: "Nuxt", className: "nuxt", src: nuxtIcon },
  php: { label: "PHP", title: "PHP", className: "php", src: phpIcon },
  python: { label: "Py", title: "Python", className: "python", src: pythonIcon },
  react: { label: "R", title: "React", className: "react", src: reactIcon },
  slim: { label: "S", title: "Slim", className: "slim", src: slimIcon },
  svelte: { label: "S", title: "Svelte", className: "svelte", src: svelteIcon },
  symfony: { label: "Sf", title: "Symfony", className: "symfony", src: symfonyIcon },
  typescript: { label: "TS", title: "TypeScript", className: "typescript", src: typescriptIcon },
  vue: { label: "V", title: "Vue", className: "vue", src: vueIcon },
};

const PROJECT_TEMPLATES: ProjectTemplate[] = [
  {
    cardId: "dart-console",
    title: "Dart",
    framework: "Dart",
    language: "Dart",
    description: "Minimal Dart console app generated with Dart CLI.",
    tags: ["Dart", "CLI", "Pub"],
    versions: [
      {
        id: "dart-console-latest",
        label: "Latest",
        command: "dart create -t console-simple my-app",
      },
    ],
  },
  {
    cardId: "flutter-app",
    title: "Flutter",
    framework: "Flutter",
    language: "Dart",
    description: "Cross-platform Flutter app generated with Flutter CLI.",
    tags: ["Flutter", "Dart", "Mobile"],
    versions: [
      {
        id: "flutter-app-latest",
        label: "Latest",
        command: "flutter create my-app",
      },
    ],
  },
  {
    cardId: "python-basic",
    title: "Python",
    framework: "Python",
    language: "Python",
    description: "Minimal Python app with a local virtual environment.",
    tags: ["Python", "CLI", "venv"],
    versions: [
      {
        id: "python-basic",
        label: "Latest",
        command: "python -m venv .venv && python main.py",
      },
    ],
  },
  {
    cardId: "fastapi-python",
    title: "FastAPI",
    framework: "FastAPI",
    language: "Python",
    description: "Python API starter with FastAPI and Uvicorn.",
    tags: ["Python", "FastAPI", "API"],
    versions: [
      {
        id: "fastapi-python-latest",
        label: "Latest",
        command: "python -m venv .venv && pip install fastapi uvicorn[standard]",
      },
    ],
  },
  {
    cardId: "flask-python",
    title: "Flask",
    framework: "Flask",
    language: "Python",
    description: "Lightweight Flask web app starter.",
    tags: ["Python", "Flask", "Web"],
    versions: [
      {
        id: "flask-python-latest",
        label: "Latest",
        command: "python -m venv .venv && pip install flask",
      },
    ],
  },
  {
    cardId: "django-python",
    title: "Django",
    framework: "Django",
    language: "Python",
    description: "Django web app starter with runserver scripts.",
    tags: ["Python", "Django", "MVC"],
    versions: [
      {
        id: "django-python-latest",
        label: "Latest",
        command: "python -m venv .venv && pip install django>=4.2,<5.0",
      },
    ],
  },
  {
    cardId: "node-ts",
    title: "Node.js",
    framework: "Node.js",
    language: "TypeScript",
    description: "Minimal Node.js HTTP server with TypeScript.",
    tags: ["Node", "Backend", "HTTP"],
    versions: [
      {
        id: "node-ts",
        label: "Latest",
        command: "node <scaffold> my-app && npm install -D typescript tsx @types/node",
      },
    ],
  },
  {
    cardId: "node-js",
    title: "Node.js",
    framework: "Node.js",
    language: "JavaScript",
    description: "Minimal Node.js HTTP server starter.",
    tags: ["Node", "Backend", "HTTP"],
    versions: [
      {
        id: "node-js",
        label: "Latest",
        command: "node <scaffold> my-app && npm install",
      },
    ],
  },
  {
    cardId: "express-ts",
    title: "Express.js",
    framework: "Express",
    language: "TypeScript",
    description: "Express API starter with TypeScript tooling.",
    tags: ["Node", "Express", "API"],
    versions: [
      {
        id: "express-ts",
        label: "Latest",
        command: "node <scaffold> my-app && npm install express && npm install -D typescript tsx @types/node @types/express",
      },
    ],
  },
  {
    cardId: "express-js",
    title: "Express.js",
    framework: "Express",
    language: "JavaScript",
    description: "Express API starter for Node.js.",
    tags: ["Node", "Express", "API"],
    versions: [
      {
        id: "express-js",
        label: "Latest",
        command: "node <scaffold> my-app && npm install express",
      },
    ],
  },
  {
    cardId: "nestjs-ts",
    title: "NestJS",
    framework: "NestJS",
    language: "TypeScript",
    description: "Structured NestJS backend generated with the Nest CLI.",
    tags: ["Node", "NestJS", "API"],
    versions: [
      {
        id: "nestjs-ts-latest",
        label: "Latest",
        command: "npx @nestjs/cli@latest new my-app --package-manager npm --skip-git --language TS --strict",
      },
    ],
  },
  {
    cardId: "nestjs-js",
    title: "NestJS",
    framework: "NestJS",
    language: "JavaScript",
    description: "Structured NestJS backend generated with the Nest CLI.",
    tags: ["Node", "NestJS", "API"],
    versions: [
      {
        id: "nestjs-js-latest",
        label: "Latest",
        command: "npx @nestjs/cli@latest new my-app --package-manager npm --skip-git --language JS",
      },
    ],
  },
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
    cardId: "nuxt-ts",
    title: "Nuxt",
    framework: "Nuxt",
    language: "TypeScript",
    description: "Vue full-stack app generated with Nuxt.",
    tags: ["Vue", "Nuxt", "SSR"],
    versions: [
      {
        id: "nuxt-ts-latest",
        label: "Latest",
        command: "npx nuxi@latest init my-app --template minimal --packageManager npm --gitInit=false",
      },
    ],
  },
  {
    cardId: "nuxt-js",
    title: "Nuxt",
    framework: "Nuxt",
    language: "JavaScript",
    description: "Vue full-stack app generated with Nuxt.",
    tags: ["Vue", "Nuxt", "SSR"],
    versions: [
      {
        id: "nuxt-js-latest",
        label: "Latest",
        command: "npx nuxi@latest init my-app --template minimal --packageManager npm --gitInit=false",
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
    cardId: "laravel-php",
    title: "Laravel",
    framework: "Laravel",
    language: "PHP",
    description: "Full-stack PHP framework with Composer scripts.",
    tags: ["PHP", "Composer", "MVC"],
    versions: [
      {
        id: "laravel-php-latest",
        label: "Latest",
        command: "composer create-project --no-interaction --no-progress laravel/laravel my-app",
      },
    ],
  },
  {
    cardId: "symfony-php",
    title: "Symfony",
    framework: "Symfony",
    language: "PHP",
    description: "Symfony skeleton for modular PHP applications.",
    tags: ["PHP", "Composer", "Skeleton"],
    versions: [
      {
        id: "symfony-php-latest",
        label: "Latest",
        command: "composer create-project --no-interaction --no-progress symfony/skeleton my-app",
      },
    ],
  },
  {
    cardId: "slim-php",
    title: "Slim",
    framework: "Slim",
    language: "PHP",
    description: "Small PHP framework for APIs and lightweight apps.",
    tags: ["PHP", "Composer", "API"],
    versions: [
      {
        id: "slim-php-latest",
        label: "Latest",
        command: "composer create-project --no-interaction --no-progress slim/slim-skeleton my-app",
      },
    ],
  },
  {
    cardId: "codeigniter-php",
    title: "CodeIgniter",
    framework: "CodeIgniter",
    language: "PHP",
    description: "CodeIgniter 4 app starter generated with Composer.",
    tags: ["PHP", "Composer", "MVC"],
    versions: [
      {
        id: "codeigniter-php-latest",
        label: "Latest",
        command: "composer create-project --no-interaction --no-progress codeigniter4/appstarter my-app",
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

function templateIconKey(template: ProjectTemplate): TemplateIconKey {
  if (template.cardId.includes("flutter")) return "flutter";
  if (template.cardId.includes("dart")) return "dart";
  if (template.cardId.includes("fastapi")) return "fastapi";
  if (template.cardId.includes("flask")) return "flask";
  if (template.cardId.includes("django")) return "django";
  if (template.cardId.includes("node")) return "node";
  if (template.cardId.includes("express")) return "express";
  if (template.cardId.includes("nestjs")) return "nestjs";
  if (template.cardId.includes("react") || template.cardId.includes("cra")) return "react";
  if (template.cardId.includes("next")) return "next";
  if (template.cardId.includes("vue")) return "vue";
  if (template.cardId.includes("nuxt")) return "nuxt";
  if (template.cardId.includes("svelte")) return "svelte";
  if (template.cardId.includes("laravel")) return "laravel";
  if (template.cardId.includes("symfony")) return "symfony";
  if (template.cardId.includes("slim")) return "slim";
  if (template.cardId.includes("codeigniter")) return "codeigniter";
  if (template.cardId.includes("angular")) return "angular";
  if (template.language === "Python") return "python";
  if (template.language === "PHP") return "php";
  if (template.language === "TypeScript") return "typescript";
  return "javascript";
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
              {(["All", "JavaScript", "TypeScript", "Python", "PHP", "Dart"] as LanguageFilter[]).map((item) => (
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
                <div key={`${index}-${log.text}`} className={`create-log-line ${log.isError ? "error" : ""}`}>
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
