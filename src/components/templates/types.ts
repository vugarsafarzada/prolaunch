// Shared template types for the Create Project flow.

export type LanguageFilter =
  | "All"
  | "JavaScript"
  | "TypeScript"
  | "Python"
  | "PHP"
  | "Dart"
  | "Go"
  | "Java"
  | "Ruby";

export interface TemplateVersion {
  id: string;
  label: string;
  command: string;
}

export interface ProjectTemplate {
  cardId: string;
  title: string;
  framework: string;
  language: Exclude<LanguageFilter, "All">;
  description: string;
  tags: string[];
  versions: TemplateVersion[];
}

export type TemplateIconKey =
  | "angular"
  | "chi"
  | "codeigniter"
  | "dart"
  | "django"
  | "echo"
  | "express"
  | "fastapi"
  | "fiber"
  | "flutter"
  | "flask"
  | "gin"
  | "go"
  | "gradle"
  | "java"
  | "javascript"
  | "laravel"
  | "maven"
  | "nestjs"
  | "next"
  | "node"
  | "nuxt"
  | "php"
  | "python"
  | "rails"
  | "react"
  | "reactNative"
  | "ruby"
  | "sinatra"
  | "slim"
  | "svelte"
  | "springBoot"
  | "symfony"
  | "typescript"
  | "vue";

export interface TemplateIconMeta {
  label: string;
  title: string;
  className: string;
  src?: string;
}
