// Joins every per-language template list into one PROJECT_TEMPLATES array and
// re-exports the shared template types and icon helpers.

import type { ProjectTemplate } from "./types";
import { rubyTemplates } from "./ruby";
import { javaTemplates } from "./java";
import { goTemplates } from "./go";
import { dartTemplates } from "./dart";
import { pythonTemplates } from "./python";
import { nodeTemplates } from "./node";
import { phpTemplates } from "./php";

export const PROJECT_TEMPLATES: ProjectTemplate[] = [
  ...rubyTemplates,
  ...javaTemplates,
  ...goTemplates,
  ...dartTemplates,
  ...pythonTemplates,
  ...nodeTemplates,
  ...phpTemplates,
];

export type {
  LanguageFilter,
  TemplateVersion,
  ProjectTemplate,
  TemplateIconKey,
  TemplateIconMeta,
} from "./types";
export { TEMPLATE_ICONS, templateIconKey } from "./icons";
