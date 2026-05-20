#![allow(unused_imports)]
//! Go support: toolchain detection, framework scaffolds, go.mod project reading.

use std::path::{Path, PathBuf};

use crate::common::*;

pub(crate) fn go_bin() -> String {
    if cfg!(target_os = "windows") {
        "go.exe".to_string()
    } else {
        "go".to_string()
    }
}

pub(crate) fn go_main_source(kind: &str) -> &'static str {
    match kind {
        "gin" => {
            r#"package main

import "github.com/gin-gonic/gin"

func main() {
	router := gin.Default()

	router.GET("/", func(context *gin.Context) {
		context.JSON(200, gin.H{"message": "Hello from Gin"})
	})

	router.Run(":8080")
}
"#
        }
        "fiber" => {
            r#"package main

import "github.com/gofiber/fiber/v2"

func main() {
	app := fiber.New()

	app.Get("/", func(context *fiber.Ctx) error {
		return context.JSON(fiber.Map{"message": "Hello from Fiber"})
	})

	app.Listen(":3000")
}
"#
        }
        "echo" => {
            r#"package main

import (
	"net/http"

	"github.com/labstack/echo/v4"
)

func main() {
	app := echo.New()

	app.GET("/", func(context echo.Context) error {
		return context.JSON(http.StatusOK, map[string]string{"message": "Hello from Echo"})
	})

	app.Logger.Fatal(app.Start(":1323"))
}
"#
        }
        "chi" => {
            r#"package main

import (
	"encoding/json"
	"net/http"

	"github.com/go-chi/chi/v5"
)

func main() {
	router := chi.NewRouter()

	router.Get("/", func(writer http.ResponseWriter, request *http.Request) {
		writer.Header().Set("Content-Type", "application/json")
		json.NewEncoder(writer).Encode(map[string]string{"message": "Hello from Chi"})
	})

	http.ListenAndServe(":3000", router)
}
"#
        }
        _ => {
            r#"package main

import "fmt"

func main() {
	fmt.Println("Hello from Go")
}
"#
        }
    }
}

pub(crate) fn go_dependency(kind: &str) -> Option<&'static str> {
    match kind {
        "gin" => Some("github.com/gin-gonic/gin"),
        "fiber" => Some("github.com/gofiber/fiber/v2"),
        "echo" => Some("github.com/labstack/echo/v4"),
        "chi" => Some("github.com/go-chi/chi/v5"),
        _ => None,
    }
}

pub(crate) fn go_project_steps(
    kind: &str,
    project_name: &str,
    parent_dir: &Path,
    target_dir: &Path,
) -> Vec<CreationStep> {
    let title = match kind {
        "gin" => "Gin",
        "fiber" => "Fiber",
        "echo" => "Echo",
        "chi" => "Chi",
        _ => "Go",
    };

    let mut steps = vec![
        write_files_step(
            &format!("Create {} starter files", title),
            vec![
                (target_dir.join("main.go"), go_main_source(kind).to_string()),
                (
                    target_dir.join("README.md"),
                    format!("# {}\n\nCreated with ProLaunch.\n", project_name),
                ),
            ],
            parent_dir,
            format!("$ ProLaunch scaffold {}", project_name),
        ),
        creation_step(
            "Initialize Go module",
            go_bin(),
            vec!["mod", "init", project_name],
            target_dir,
        ),
    ];

    if let Some(dependency) = go_dependency(kind) {
        steps.push(creation_step_from_strings(
            &format!("Install {} dependency", title),
            go_bin(),
            vec!["get".to_string(), dependency.to_string()],
            target_dir,
        ));
    }

    steps.push(creation_step(
        "Tidy Go module",
        go_bin(),
        vec!["mod", "tidy"],
        target_dir,
    ));

    steps
}

pub(crate) fn go_module_name(project_dir: &Path, content: &str) -> String {
    content
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("module ")
                .map(str::trim)
                .filter(|module| !module.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| python_folder_project_name(project_dir))
}

pub(crate) fn has_go_main_package(project_dir: &Path) -> bool {
    let entries = match std::fs::read_dir(project_dir) {
        Ok(entries) => entries,
        Err(_) => return project_dir.join("main.go").exists(),
    };

    entries.filter_map(Result::ok).any(|entry| {
        let path = entry.path();
        let is_go_file = path.extension().and_then(|ext| ext.to_str()) == Some("go");
        let is_test_file = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.ends_with("_test.go"))
            .unwrap_or(false);

        is_go_file
            && !is_test_file
            && std::fs::read_to_string(path)
                .map(|content| content.contains("package main"))
                .unwrap_or(false)
    })
}

pub(crate) fn go_scripts(project_dir: &Path, has_module: bool) -> Vec<ScriptInfo> {
    let source = if has_module { "go.mod" } else { "main.go" };
    let run_command = if has_module {
        "go run ."
    } else {
        "go run main.go"
    };
    let mut scripts = Vec::new();

    if has_module || project_dir.join("main.go").exists() {
        scripts.push(("start", run_command));
        scripts.push(("dev", run_command));
    }

    if has_module {
        scripts.push(("test", "go test ./..."));
        scripts.push(("tidy", "go mod tidy"));
        scripts.push(("build", "go build ."));
    }

    scripts
        .into_iter()
        .map(|(name, command)| ScriptInfo {
            name: name.to_string(),
            command: command.to_string(),
            package_manager: Some("go".to_string()),
            source: Some(source.to_string()),
        })
        .collect()
}

pub(crate) fn go_main_scripts(project_dir: &Path) -> Vec<ScriptInfo> {
    go_scripts(project_dir, false)
}

pub(crate) fn read_go_project(
    project_path: String,
    project_dir: &Path,
    go_mod_path: &Path,
) -> Result<ProjectInfo, String> {
    let content = std::fs::read_to_string(go_mod_path)
        .map_err(|e| format!("Failed to read go.mod: {}", e))?;

    let mut scripts = go_scripts(project_dir, true);
    if !has_go_main_package(project_dir) {
        scripts.retain(|script| script.name != "start" && script.name != "dev");
    }

    Ok(ProjectInfo {
        name: go_module_name(project_dir, &content),
        path: project_path,
        scripts,
        package_manager: "go".to_string(),
    })
}

pub(crate) fn go_script_command_line(
    project_dir: &Path,
    script_name: &str,
) -> Result<String, String> {
    let has_module = project_dir.join("go.mod").exists();
    let mut scripts = if has_module {
        go_scripts(project_dir, true)
    } else if project_dir.join("main.go").exists() {
        go_main_scripts(project_dir)
    } else {
        Vec::new()
    };

    if has_module && !has_go_main_package(project_dir) {
        scripts.retain(|script| script.name != "start" && script.name != "dev");
    }

    scripts
        .into_iter()
        .find(|script| script.name == script_name)
        .map(|script| script.command)
        .ok_or_else(|| format!("Go script '{}' was not found", script_name))
}
