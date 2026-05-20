import type { ProjectTemplate } from "./types";

export const goTemplates: ProjectTemplate[] = [
  {
    cardId: "go-basic",
    title: "Go",
    framework: "Go",
    language: "Go",
    description: "Minimal Go app with module setup and run scripts.",
    tags: ["Go", "CLI", "Module"],
    versions: [
      {
        id: "go-basic-latest",
        label: "Latest",
        command: "go mod init my-app && go run .",
      },
    ],
  },
  {
    cardId: "gin-go",
    title: "Gin",
    framework: "Gin",
    language: "Go",
    description: "Go HTTP API starter using the Gin web framework.",
    tags: ["Go", "Gin", "API"],
    versions: [
      {
        id: "gin-go-latest",
        label: "Latest",
        command: "go mod init my-app && go get github.com/gin-gonic/gin",
      },
    ],
  },
  {
    cardId: "fiber-go",
    title: "Fiber",
    framework: "Fiber",
    language: "Go",
    description: "Fast Go web app starter using Fiber.",
    tags: ["Go", "Fiber", "Web"],
    versions: [
      {
        id: "fiber-go-latest",
        label: "Latest",
        command: "go mod init my-app && go get github.com/gofiber/fiber/v2",
      },
    ],
  },
  {
    cardId: "echo-go",
    title: "Echo",
    framework: "Echo",
    language: "Go",
    description: "Go API starter with Echo routing.",
    tags: ["Go", "Echo", "API"],
    versions: [
      {
        id: "echo-go-latest",
        label: "Latest",
        command: "go mod init my-app && go get github.com/labstack/echo/v4",
      },
    ],
  },
  {
    cardId: "chi-go",
    title: "Chi",
    framework: "Chi",
    language: "Go",
    description: "Small Go router starter using Chi.",
    tags: ["Go", "Chi", "Router"],
    versions: [
      {
        id: "chi-go-latest",
        label: "Latest",
        command: "go mod init my-app && go get github.com/go-chi/chi/v5",
      },
    ],
  },
];
