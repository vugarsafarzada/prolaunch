#![allow(unused_imports)]
//! Python support: interpreter discovery, venv health checks, scaffolds, project reading.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::AppHandle;

use crate::common::*;

pub(crate) fn python_candidates() -> Vec<ToolCommand> {
    if cfg!(target_os = "windows") {
        vec![
            ToolCommand {
                program: "py".to_string(),
                args: vec!["-3".to_string()],
            },
            ToolCommand {
                program: "python".to_string(),
                args: Vec::new(),
            },
            ToolCommand {
                program: "python3".to_string(),
                args: Vec::new(),
            },
        ]
    } else {
        vec![
            ToolCommand {
                program: "python3.13".to_string(),
                args: Vec::new(),
            },
            ToolCommand {
                program: "python3.12".to_string(),
                args: Vec::new(),
            },
            ToolCommand {
                program: "python3.11".to_string(),
                args: Vec::new(),
            },
            ToolCommand {
                program: "python3.10".to_string(),
                args: Vec::new(),
            },
            ToolCommand {
                program: "python3".to_string(),
                args: Vec::new(),
            },
            ToolCommand {
                program: "/usr/bin/python3".to_string(),
                args: Vec::new(),
            },
            ToolCommand {
                program: "python".to_string(),
                args: Vec::new(),
            },
        ]
    }
}

pub(crate) fn temp_python_check_dir() -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "prolaunch-python-check-{}-{}",
        std::process::id(),
        suffix
    ))
}

pub(crate) fn check_python_health(candidate: &ToolCommand) -> Result<String, String> {
    let version_args = tool_args(candidate, &["--version"]);
    let version = command_output(&candidate.program, &version_args, None)?;

    let stdlib_args = tool_args(
        candidate,
        &[
            "-c",
            "from xml.parsers import expat; import venv; print('stdlib ok')",
        ],
    );
    command_output(&candidate.program, &stdlib_args, None)?;

    let temp_dir = temp_python_check_dir();
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to prepare Python health check: {}", e))?;

    let result = (|| {
        let venv_args = tool_args(candidate, &["-m", "venv", ".venv"]);
        command_output(&candidate.program, &venv_args, Some(&temp_dir))?;

        let venv_python = python_venv_python_path(&temp_dir);
        let venv_python = venv_python.to_string_lossy().to_string();
        let pip_args = vec!["-m".to_string(), "pip".to_string(), "--version".to_string()];
        command_output(&venv_python, &pip_args, Some(&temp_dir))?;

        let import_args = vec![
            "-c".to_string(),
            "from xml.parsers import expat; import pip; print('pip ok')".to_string(),
        ];
        command_output(&venv_python, &import_args, Some(&temp_dir))?;
        Ok(first_output_line(&version))
    })();

    let _ = std::fs::remove_dir_all(&temp_dir);
    result
}

pub(crate) fn prepare_python(app: &AppHandle, creation_id: &str) -> Result<ToolCommand, String> {
    emit_create_log(app, creation_id, "Checking Python...".to_string(), false);
    let mut errors = Vec::new();

    for candidate in python_candidates() {
        match check_python_health(&candidate) {
            Ok(version) => {
                emit_create_log(
                    app,
                    creation_id,
                    format!("Python found: {}", version),
                    false,
                );
                return Ok(candidate);
            }
            Err(details) => {
                errors.push(format!("{}: {}", candidate.program, details));
            }
        }
    }

    Err(format!(
        "A working Python with venv and pip is required for this template but was not found.\n{}",
        errors.join("\n")
    ))
}

pub(crate) fn python_scaffold_script() -> &'static str {
    r###"
from pathlib import Path
import sys
import textwrap

target = Path(sys.argv[-3])
project_name = sys.argv[-2]
kind = sys.argv[-1]

if target.exists():
    print(f"Target folder already exists: {target}", file=sys.stderr)
    sys.exit(1)

target.mkdir(parents=True)

def write(relative_path, content):
    path = target / relative_path
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(textwrap.dedent(content).lstrip(), encoding="utf-8")

def pyproject(dependencies, scripts):
    dependency_lines = "\n".join(f'  "{dependency}",' for dependency in dependencies)
    script_lines = "\n".join(f'{name} = "{command}"' for name, command in scripts.items())
    return textwrap.dedent(f"""\
    [project]
    name = "{project_name}"
    version = "0.1.0"
    requires-python = ">=3.9"
    dependencies = [
    {dependency_lines}
    ]

    [tool.prolaunch.scripts]
    {script_lines}
    """)

def requirements(dependencies):
    return "\n".join(dependencies) + ("\n" if dependencies else "")

if kind == "fastapi":
    dependencies = ["fastapi", "uvicorn[standard]"]
    scripts = {
        "dev": "python -m uvicorn app.main:app --reload",
        "start": "python -m uvicorn app.main:app"
    }
    write("app/__init__.py", "")
    write("app/main.py", """
    from fastapi import FastAPI

    app = FastAPI()

    @app.get("/")
    def read_root():
        return {"message": "Hello from FastAPI"}
    """)
elif kind == "flask":
    dependencies = ["flask"]
    scripts = {
        "dev": "python -m flask --app app run --debug",
        "start": "python -m flask --app app run"
    }
    write("app.py", """
    from flask import Flask, jsonify

    app = Flask(__name__)

    @app.get("/")
    def home():
        return jsonify(message="Hello from Flask")
    """)
elif kind == "django":
    dependencies = ["django>=4.2,<5.0"]
    scripts = {
        "dev": "python manage.py runserver",
        "start": "python manage.py runserver",
        "migrate": "python manage.py migrate"
    }
    write("manage.py", """
    #!/usr/bin/env python
    import os
    import sys

    def main():
        os.environ.setdefault("DJANGO_SETTINGS_MODULE", "config.settings")
        from django.core.management import execute_from_command_line
        execute_from_command_line(sys.argv)

    if __name__ == "__main__":
        main()
    """)
    write("config/__init__.py", "")
    write("config/settings.py", f"""
    from pathlib import Path

    BASE_DIR = Path(__file__).resolve().parent.parent
    SECRET_KEY = "prolaunch-development-key"
    DEBUG = True
    ALLOWED_HOSTS = []

    INSTALLED_APPS = [
        "django.contrib.admin",
        "django.contrib.auth",
        "django.contrib.contenttypes",
        "django.contrib.sessions",
        "django.contrib.messages",
        "django.contrib.staticfiles",
    ]

    MIDDLEWARE = [
        "django.middleware.security.SecurityMiddleware",
        "django.contrib.sessions.middleware.SessionMiddleware",
        "django.middleware.common.CommonMiddleware",
        "django.middleware.csrf.CsrfViewMiddleware",
        "django.contrib.auth.middleware.AuthenticationMiddleware",
        "django.contrib.messages.middleware.MessageMiddleware",
        "django.middleware.clickjacking.XFrameOptionsMiddleware",
    ]

    ROOT_URLCONF = "config.urls"
    TEMPLATES = [
        {{
            "BACKEND": "django.template.backends.django.DjangoTemplates",
            "DIRS": [],
            "APP_DIRS": True,
            "OPTIONS": {{
                "context_processors": [
                    "django.template.context_processors.request",
                    "django.contrib.auth.context_processors.auth",
                    "django.contrib.messages.context_processors.messages",
                ],
            }},
        }},
    ]
    WSGI_APPLICATION = "config.wsgi.application"

    DATABASES = {{
        "default": {{
            "ENGINE": "django.db.backends.sqlite3",
            "NAME": BASE_DIR / "db.sqlite3",
        }}
    }}

    LANGUAGE_CODE = "en-us"
    TIME_ZONE = "UTC"
    USE_I18N = True
    USE_TZ = True
    STATIC_URL = "static/"
    DEFAULT_AUTO_FIELD = "django.db.models.BigAutoField"
    """)
    write("config/urls.py", """
    from django.http import JsonResponse
    from django.urls import path

    def home(_request):
        return JsonResponse({"message": "Hello from Django"})

    urlpatterns = [
        path("", home),
    ]
    """)
    write("config/asgi.py", """
    import os
    from django.core.asgi import get_asgi_application

    os.environ.setdefault("DJANGO_SETTINGS_MODULE", "config.settings")
    application = get_asgi_application()
    """)
    write("config/wsgi.py", """
    import os
    from django.core.wsgi import get_wsgi_application

    os.environ.setdefault("DJANGO_SETTINGS_MODULE", "config.settings")
    application = get_wsgi_application()
    """)
else:
    dependencies = []
    scripts = {
        "dev": "python main.py",
        "start": "python main.py"
    }
    write("main.py", """
    def main():
        print("Hello from Python")

    if __name__ == "__main__":
        main()
    """)

write("pyproject.toml", pyproject(dependencies, scripts))
write("requirements.txt", requirements(dependencies))
write("README.md", f"# {project_name}\n\nCreated with ProLaunch.\n")
print(f"{kind.title()} Python starter files created")
"###
}

pub(crate) fn python_venv_python_path(project_dir: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        project_dir.join(".venv").join("Scripts").join("python.exe")
    } else {
        project_dir.join(".venv").join("bin").join("python")
    }
}

pub(crate) fn python_create_venv_script() -> &'static str {
    r###"
from pathlib import Path
import shutil
import subprocess
import sys

venv_dir = Path(".venv")

def run(args):
    subprocess.run(args, check=True)

try:
    run([sys.executable, "-m", "venv", str(venv_dir)])
    print("Python virtual environment created")
except subprocess.CalledProcessError:
    print("Standard venv creation failed; retrying without bundled pip")
    if venv_dir.exists():
        shutil.rmtree(venv_dir)
    run([
        sys.executable,
        "-m",
        "venv",
        "--system-site-packages",
        "--without-pip",
        str(venv_dir),
    ])
    print("Python virtual environment created with system site packages")
"###
}

pub(crate) fn python_project_steps(
    kind: &str,
    project_name: &str,
    parent_dir: &Path,
    target_dir: &Path,
    python: &ToolCommand,
) -> Vec<CreationStep> {
    let mut steps = vec![
        creation_step_with_display(
            "Create Python starter files",
            python.program.clone(),
            tool_args_owned(
                python,
                vec![
                    "-c".to_string(),
                    python_scaffold_script().to_string(),
                    target_dir.to_string_lossy().to_string(),
                    project_name.to_string(),
                    kind.to_string(),
                ],
            ),
            parent_dir,
            format!("$ python <ProLaunch scaffold> {}", project_name),
        ),
        creation_step_with_display(
            "Create Python virtual environment",
            python.program.clone(),
            tool_args_owned(
                python,
                vec!["-c".to_string(), python_create_venv_script().to_string()],
            ),
            target_dir,
            "$ python -m venv .venv".to_string(),
        ),
    ];

    if kind != "basic" {
        let venv_python = python_venv_python_path(target_dir);
        steps.push(creation_step_with_display(
            "Install Python dependencies",
            venv_python.to_string_lossy().to_string(),
            vec![
                "-m".to_string(),
                "pip".to_string(),
                "install".to_string(),
                "-r".to_string(),
                "requirements.txt".to_string(),
            ],
            target_dir,
            "$ .venv python -m pip install -r requirements.txt".to_string(),
        ));
    }

    steps
}

pub(crate) fn read_python_scripts(parsed: &toml::Value) -> Vec<ScriptInfo> {
    parsed
        .get("tool")
        .and_then(|tool| tool.get("prolaunch"))
        .and_then(|prolaunch| prolaunch.get("scripts"))
        .and_then(|scripts| scripts.as_table())
        .map(|scripts| {
            scripts
                .iter()
                .filter_map(|(name, command)| {
                    command.as_str().map(|command| ScriptInfo {
                        name: name.clone(),
                        command: command.to_string(),
                        package_manager: Some("python".to_string()),
                        source: Some("pyproject.toml".to_string()),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn python_main_script() -> ScriptInfo {
    ScriptInfo {
        name: "start".to_string(),
        command: "python main.py".to_string(),
        package_manager: Some("python".to_string()),
        source: Some("main.py".to_string()),
    }
}

pub(crate) fn python_project_name(project_dir: &Path, parsed: &toml::Value) -> String {
    parsed
        .get("project")
        .and_then(|project| project.get("name"))
        .and_then(|name| name.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| python_folder_project_name(project_dir))
}

pub(crate) fn read_python_project(
    project_path: String,
    project_dir: &Path,
    pyproject_path: &Path,
) -> Result<ProjectInfo, String> {
    let content = std::fs::read_to_string(pyproject_path)
        .map_err(|e| format!("Failed to read pyproject.toml: {}", e))?;

    let parsed: toml::Value =
        toml::from_str(&content).map_err(|e| format!("Invalid pyproject.toml: {}", e))?;

    Ok(ProjectInfo {
        name: python_project_name(project_dir, &parsed),
        path: project_path,
        scripts: read_python_scripts(&parsed),
        package_manager: "python".to_string(),
    })
}

pub(crate) fn python_script_command(
    project_dir: &Path,
    script_name: &str,
) -> Result<String, String> {
    let pyproject_path = project_dir.join("pyproject.toml");
    if !pyproject_path.exists() && project_dir.join("main.py").exists() {
        if script_name == "start" {
            return Ok(python_main_script().command);
        }

        return Err(format!(
            "Python script '{}' was not found. main.py projects only expose the 'start' script.",
            script_name
        ));
    }

    let content = std::fs::read_to_string(&pyproject_path)
        .map_err(|e| format!("Failed to read pyproject.toml: {}", e))?;
    let parsed: toml::Value =
        toml::from_str(&content).map_err(|e| format!("Invalid pyproject.toml: {}", e))?;

    let script = read_python_scripts(&parsed)
        .into_iter()
        .find(|script| script.name == script_name)
        .map(|script| script.command);

    if let Some(command) = script {
        return Ok(command);
    }

    if script_name == "start" && project_dir.join("main.py").exists() {
        return Ok(python_main_script().command);
    }

    Err(format!(
        "Python script '{}' was not found in pyproject.toml",
        script_name
    ))
}

pub(crate) fn project_python_command(project_dir: &Path) -> Result<ToolCommand, String> {
    let venv_python = python_venv_python_path(project_dir);
    if venv_python.exists() {
        return Ok(ToolCommand {
            program: venv_python.to_string_lossy().to_string(),
            args: Vec::new(),
        });
    }

    for candidate in python_candidates() {
        let args = tool_args(&candidate, &["--version"]);
        if command_output(&candidate.program, &args, None).is_ok() {
            return Ok(candidate);
        }
    }

    Err("Python is required to run this script but was not found.".to_string())
}

pub(crate) fn python_script_command_line(
    project_dir: &Path,
    script_name: &str,
) -> Result<String, String> {
    let command = python_script_command(project_dir, script_name)?;
    let python = project_python_command(project_dir)?;
    let python_prefix = tool_command_line(&python, &[]);

    if command == "python" {
        Ok(python_prefix)
    } else if let Some(rest) = command.strip_prefix("python ") {
        Ok(format!("{} {}", python_prefix, rest))
    } else {
        Ok(command)
    }
}
