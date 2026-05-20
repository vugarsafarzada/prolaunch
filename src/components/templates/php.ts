import type { ProjectTemplate } from "./types";

export const phpTemplates: ProjectTemplate[] = [
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
];
