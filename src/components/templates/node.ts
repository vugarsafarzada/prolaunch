import type { ProjectTemplate } from "./types";

// Node.js ecosystem templates (JavaScript and TypeScript): backend runtimes,
// React / Vue / Svelte / Angular frontend frameworks and meta-frameworks.
export const nodeTemplates: ProjectTemplate[] = [
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
  {
    cardId: "react-native-ts",
    title: "React Native",
    framework: "React Native",
    language: "TypeScript",
    description: "Cross-platform React Native app scaffolded with Expo.",
    tags: ["React Native", "Expo", "Mobile"],
    versions: [
      {
        id: "react-native-ts",
        label: "Latest",
        command: "npx create-expo-app@latest my-app --template blank-typescript",
      },
    ],
  },
  {
    cardId: "react-native-js",
    title: "React Native",
    framework: "React Native",
    language: "JavaScript",
    description: "Cross-platform React Native app scaffolded with Expo.",
    tags: ["React Native", "Expo", "Mobile"],
    versions: [
      {
        id: "react-native-js",
        label: "Latest",
        command: "npx create-expo-app@latest my-app --template blank",
      },
    ],
  },
];
