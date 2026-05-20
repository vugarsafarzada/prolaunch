#![allow(unused_imports)]
//! Node.js / JavaScript / TypeScript support: npm tooling, scaffolds, project reading.

use std::path::Path;

use crate::common::*;
use crate::preferred_package_manager;

pub(crate) fn npm_bin() -> String {
    if cfg!(target_os = "windows") {
        "npm.cmd".to_string()
    } else {
        "npm".to_string()
    }
}

pub(crate) fn npx_bin() -> String {
    if cfg!(target_os = "windows") {
        "npx.cmd".to_string()
    } else {
        "npx".to_string()
    }
}

pub(crate) fn node_bin() -> String {
    "node".to_string()
}

pub(crate) fn preferred_node_package_manager(path: &Path) -> String {
    if path.join("pnpm-lock.yaml").exists() {
        "pnpm".to_string()
    } else if path.join("yarn.lock").exists() || path.join(".yarnrc.yml").exists() {
        "yarn".to_string()
    } else if path.join("bun.lockb").exists() || path.join("bun.lock").exists() {
        "bun".to_string()
    } else {
        "npm".to_string()
    }
}

pub(crate) fn vite_steps(
    template: &str,
    project_name: &str,
    parent_dir: &Path,
    target_dir: &Path,
) -> Vec<CreationStep> {
    vec![
        creation_step(
            "Create Vite project",
            npx_bin(),
            vec![
                "-y",
                "create-vite@latest",
                project_name,
                "--template",
                template,
                "--no-interactive",
            ],
            parent_dir,
        ),
        creation_step(
            "Install dependencies",
            npm_bin(),
            vec!["install"],
            target_dir,
        ),
    ]
}

pub(crate) fn next_steps(
    version: &str,
    use_typescript: bool,
    project_name: &str,
    parent_dir: &Path,
) -> Vec<CreationStep> {
    let package = if version == "latest" {
        "create-next-app@latest".to_string()
    } else {
        format!("create-next-app@{}", version)
    };
    let language_flag = if use_typescript { "--ts" } else { "--js" };
    let mut args = vec![
        "-y".to_string(),
        package,
        project_name.to_string(),
        language_flag.to_string(),
        "--eslint".to_string(),
        "--tailwind".to_string(),
        "--app".to_string(),
        "--src-dir".to_string(),
        "--import-alias".to_string(),
        "@/*".to_string(),
        "--use-npm".to_string(),
    ];

    if version != "14" {
        args.push("--yes".to_string());
        args.push("--disable-git".to_string());
    }

    vec![creation_step_from_strings(
        "Create Next.js project",
        npx_bin(),
        args,
        parent_dir,
    )]
}

pub(crate) fn cra_steps(
    use_typescript: bool,
    project_name: &str,
    parent_dir: &Path,
) -> Vec<CreationStep> {
    let mut args = vec![
        "-y".to_string(),
        "create-react-app@latest".to_string(),
        project_name.to_string(),
    ];
    if use_typescript {
        args.push("--template".to_string());
        args.push("typescript".to_string());
    }

    vec![creation_step_from_strings(
        "Create React App project",
        npx_bin(),
        args,
        parent_dir,
    )]
}

pub(crate) fn nuxt_steps(project_name: &str, parent_dir: &Path) -> Vec<CreationStep> {
    vec![creation_step(
        "Create Nuxt project",
        npx_bin(),
        vec![
            "-y",
            "nuxi@latest",
            "init",
            project_name,
            "--template",
            "minimal",
            "--packageManager",
            "npm",
            "--gitInit=false",
        ],
        parent_dir,
    )]
}

pub(crate) fn angular_steps(project_name: &str, parent_dir: &Path) -> Vec<CreationStep> {
    vec![creation_step(
        "Create Angular project",
        npx_bin(),
        vec![
            "-y",
            "@angular/cli@latest",
            "new",
            project_name,
            "--directory",
            project_name,
            "--defaults",
            "--interactive=false",
            "--package-manager",
            "npm",
            "--skip-git",
            "--style",
            "css",
            "--routing=false",
        ],
        parent_dir,
    )]
}

pub(crate) fn node_backend_scaffold_script() -> &'static str {
    r#"
const fs = require("node:fs");
const path = require("node:path");

const [target, projectName, kind, language] = process.argv.slice(-4);
const isTypescript = language === "ts";
const isExpress = kind === "express";

if (!target || !projectName || !kind || !language) {
  console.error("Missing scaffold arguments");
  process.exit(1);
}

if (fs.existsSync(target)) {
  console.error(`Target folder already exists: ${target}`);
  process.exit(1);
}

fs.mkdirSync(path.join(target, "src"), { recursive: true });

const packageJson = {
  name: projectName,
  version: "0.1.0",
  private: true,
  type: "module",
  scripts: isTypescript
    ? {
        dev: "tsx watch src/index.ts",
        build: "tsc",
        start: "node dist/index.js"
      }
    : {
        dev: "node src/index.js",
        start: "node src/index.js"
      }
};

fs.writeFileSync(
  path.join(target, "package.json"),
  `${JSON.stringify(packageJson, null, 2)}\n`
);

if (isTypescript) {
  fs.writeFileSync(
    path.join(target, "tsconfig.json"),
    `${JSON.stringify(
      {
        compilerOptions: {
          target: "ES2022",
          module: "NodeNext",
          moduleResolution: "NodeNext",
          strict: true,
          esModuleInterop: true,
          skipLibCheck: true,
          forceConsistentCasingInFileNames: true,
          outDir: "dist",
          rootDir: "src"
        },
        include: ["src"]
      },
      null,
      2
    )}\n`
  );
}

const extension = isTypescript ? "ts" : "js";
const source = isExpress
  ? `import express from "express";

const app = express();
const port = Number(process.env.PORT ?? 3000);

app.get("/", (_req${isTypescript ? ": express.Request" : ""}, res${isTypescript ? ": express.Response" : ""}) => {
  res.json({ message: "Hello from Express" });
});

app.listen(port, () => {
  console.log(\`Express server running at http://localhost:\${port}\`);
});
`
  : `import http from "node:http";

const port = Number(process.env.PORT ?? 3000);

const server = http.createServer((_req, res) => {
  res.writeHead(200, { "content-type": "application/json" });
  res.end(JSON.stringify({ message: "Hello from Node.js" }));
});

server.listen(port, () => {
  console.log(\`Node server running at http://localhost:\${port}\`);
});
`;

fs.writeFileSync(path.join(target, "src", `index.${extension}`), source);
console.log(`${isExpress ? "Express" : "Node.js"} ${isTypescript ? "TypeScript" : "JavaScript"} starter files created`);
"#
}

pub(crate) fn node_backend_steps(
    kind: &str,
    use_typescript: bool,
    project_name: &str,
    target_dir: &Path,
) -> Vec<CreationStep> {
    let language = if use_typescript { "ts" } else { "js" };
    let mut steps = vec![creation_step_with_display(
        if kind == "express" {
            "Create Express starter files"
        } else {
            "Create Node.js starter files"
        },
        node_bin(),
        vec![
            "-e".to_string(),
            node_backend_scaffold_script().to_string(),
            target_dir.to_string_lossy().to_string(),
            project_name.to_string(),
            kind.to_string(),
            language.to_string(),
        ],
        target_dir.parent().unwrap_or_else(|| Path::new(".")),
        format!(
            "$ node <ProLaunch scaffold> {} {}",
            project_name,
            if use_typescript {
                "typescript"
            } else {
                "javascript"
            }
        ),
    )];

    if kind == "express" {
        steps.push(creation_step(
            "Install Express",
            npm_bin(),
            vec!["install", "express"],
            target_dir,
        ));
    } else if !use_typescript {
        steps.push(creation_step(
            "Prepare npm lockfile",
            npm_bin(),
            vec!["install"],
            target_dir,
        ));
    }

    if use_typescript {
        let mut args = vec!["install", "--save-dev", "typescript", "tsx", "@types/node"];
        if kind == "express" {
            args.push("@types/express");
        }
        steps.push(creation_step(
            "Install TypeScript tools",
            npm_bin(),
            args,
            target_dir,
        ));
    }

    steps
}

pub(crate) fn nest_steps(
    use_typescript: bool,
    project_name: &str,
    parent_dir: &Path,
) -> Vec<CreationStep> {
    let language = if use_typescript { "TS" } else { "JS" };
    let mut args = vec![
        "-y".to_string(),
        "@nestjs/cli@latest".to_string(),
        "new".to_string(),
        project_name.to_string(),
        "--package-manager".to_string(),
        "npm".to_string(),
        "--skip-git".to_string(),
        "--language".to_string(),
        language.to_string(),
    ];

    if use_typescript {
        args.push("--strict".to_string());
    }

    vec![creation_step_from_strings(
        "Create NestJS project",
        npx_bin(),
        args,
        parent_dir,
    )]
}

pub(crate) fn read_node_project(
    project_path: String,
    project_dir: &Path,
    package_path: &Path,
) -> Result<ProjectInfo, String> {
    let content = std::fs::read_to_string(&package_path)
        .map_err(|e| format!("Failed to read package.json: {}", e))?;

    let parsed: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Invalid package.json: {}", e))?;

    let project_name = parsed
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("Untitled Project")
        .to_string();

    let scripts: Vec<ScriptInfo> = parsed
        .get("scripts")
        .and_then(|s| s.as_object())
        .map(|scripts_obj| {
            scripts_obj
                .iter()
                .map(|(name, cmd)| ScriptInfo {
                    name: name.clone(),
                    command: cmd.as_str().unwrap_or("").to_string(),
                    package_manager: Some(preferred_node_package_manager(project_dir)),
                    source: Some("package.json".to_string()),
                })
                .collect()
        })
        .unwrap_or_default();

    let package_manager = preferred_package_manager(project_dir);

    Ok(ProjectInfo {
        name: project_name,
        path: project_path,
        scripts,
        package_manager,
    })
}
