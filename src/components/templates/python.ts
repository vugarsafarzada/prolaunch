import type { ProjectTemplate } from "./types";

export const pythonTemplates: ProjectTemplate[] = [
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
];
