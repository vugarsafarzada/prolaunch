import type { ProjectTemplate } from "./types";

export const dartTemplates: ProjectTemplate[] = [
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
];
