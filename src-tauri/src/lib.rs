#![allow(unused_imports)]
//! ProLaunch backend entry point.
//!
//! Shared infrastructure lives in [`common`]; per-language support lives under
//! [`langs`] (one module each: node, php, python, dart, go, java, ruby). This
//! file keeps the cross-language dispatchers, the Tauri commands and [`run`],
//! joining every module back together.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

mod common;
mod langs;

use common::*;
use langs::dart::*;
use langs::go::*;
use langs::java::*;
use langs::node::*;
use langs::php::*;
use langs::python::*;
use langs::ruby::*;

fn script_invocation(
    app: &AppHandle,
    package_manager: &str,
    project_path: &str,
    script_name: &str,
) -> Result<ToolCommand, String> {
    let run_command = package_manager_run_command(package_manager).to_string();

    if package_manager == "custom" {
        let command_line = custom_script_command(app, project_path, script_name)?;
        return Ok(shell_tool_command(command_line));
    }

    if package_manager == "composer" {
        let system_composer = composer_bin();
        if command_output(&system_composer, &["--version".to_string()], None).is_ok() {
            return Ok(ToolCommand {
                program: system_composer,
                args: vec![run_command, script_name.to_string()],
            });
        }

        let composer_path = app_managed_composer_path(app)?;
        let composer_path_arg = composer_path.to_string_lossy().to_string();
        if composer_path.exists()
            && command_output(
                &php_bin(),
                &[composer_path_arg.clone(), "--version".to_string()],
                None,
            )
            .is_ok()
        {
            return Ok(ToolCommand {
                program: php_bin(),
                args: vec![composer_path_arg, run_command, script_name.to_string()],
            });
        }

        return Err(
            "Composer is required to run this PHP script. Create a PHP project once with ProLaunch setup, or install Composer on PATH."
                .to_string(),
        );
    }

    if package_manager == "python" {
        let project_dir = Path::new(project_path);
        let command_line = python_script_command_line(project_dir, script_name)?;
        return Ok(shell_tool_command(command_line));
    }

    if package_manager == "dart" || package_manager == "flutter" {
        let project_dir = Path::new(project_path);
        let command_line = pub_script_command_line(project_dir, package_manager, script_name)?;
        return Ok(shell_tool_command(command_line));
    }

    if package_manager == "go" {
        let project_dir = Path::new(project_path);
        let command_line = go_script_command_line(project_dir, script_name)?;
        return Ok(shell_tool_command(command_line));
    }

    if package_manager == "java" {
        let project_dir = Path::new(project_path);
        let command_line = java_script_command_line(project_dir, script_name)?;
        return Ok(shell_tool_command(command_line));
    }

    if package_manager == "maven" {
        let project_dir = Path::new(project_path);
        let command_line = maven_script_command_line(project_dir, script_name)?;
        return Ok(shell_tool_command(command_line));
    }

    if package_manager == "gradle" {
        let project_dir = Path::new(project_path);
        let command_line = gradle_script_command_line(project_dir, script_name)?;
        return Ok(shell_tool_command(command_line));
    }

    if package_manager == "ruby" {
        let project_dir = Path::new(project_path);
        let command_line = ruby_script_command_line(project_dir, script_name)?;
        return Ok(shell_tool_command(command_line));
    }

    Ok(ToolCommand {
        program: package_manager_command(package_manager),
        args: vec![run_command, script_name.to_string()],
    })
}

fn script_command(
    app: &AppHandle,
    package_manager: &str,
    project_path: &str,
    script_name: &str,
) -> Result<Command, String> {
    let invocation = script_invocation(app, package_manager, project_path, script_name)?;
    let parent_pid = std::process::id().to_string();

    #[cfg(unix)]
    {
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(creation_parent_watch_script())
            .arg("prolaunch-runner")
            .arg(parent_pid)
            .arg(&invocation.program)
            .args(&invocation.args);
        Ok(cmd)
    }

    #[cfg(windows)]
    {
        let mut cmd = Command::new("powershell.exe");
        cmd.arg("-NoLogo")
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-Command")
            .arg(creation_parent_watch_script())
            .arg(parent_pid)
            .arg(&invocation.program)
            .args(&invocation.args);
        Ok(cmd)
    }
}

pub(crate) fn preferred_package_manager(path: &Path) -> String {
    if path.join("composer.json").exists() {
        "composer".to_string()
    } else if path.join("pyproject.toml").exists() {
        "python".to_string()
    } else if path.join("main.py").exists() {
        "python".to_string()
    } else if path.join("go.mod").exists() || path.join("main.go").exists() {
        "go".to_string()
    } else if path.join("pom.xml").exists() {
        "maven".to_string()
    } else if path.join("build.gradle").exists() || path.join("build.gradle.kts").exists() {
        "gradle".to_string()
    } else if find_java_main_file(path).is_some() {
        "java".to_string()
    } else if path.join("Gemfile").exists() || path.join("main.rb").exists() {
        "ruby".to_string()
    } else if path.join("pubspec.yaml").exists() {
        preferred_pub_package_manager(path)
    } else if path.join("pnpm-lock.yaml").exists() {
        "pnpm".to_string()
    } else if path.join("yarn.lock").exists() || path.join(".yarnrc.yml").exists() {
        "yarn".to_string()
    } else if path.join("bun.lockb").exists() || path.join("bun.lock").exists() {
        "bun".to_string()
    } else {
        "npm".to_string()
    }
}

fn normalize_package_manager(package_manager: Option<String>, project_path: &str) -> String {
    match package_manager.as_deref() {
        Some("npm") | Some("pnpm") | Some("yarn") | Some("bun") | Some("composer")
        | Some("python") | Some("dart") | Some("flutter") | Some("go") | Some("java")
        | Some("maven") | Some("gradle") | Some("ruby") | Some("custom") => {
            package_manager.unwrap()
        }
        _ => preferred_package_manager(std::path::Path::new(project_path)),
    }
}

fn template_requirements(template_id: &str) -> Result<Vec<ToolRequirement>, String> {
    let requirements = match template_id {
        "node-ts" | "node-js" | "express-ts" | "express-js" | "vite-react-ts" | "vite-react-js"
        | "vite-vue-ts" | "vite-vue-js" | "nuxt-ts-latest" | "nuxt-js-latest"
        | "vite-svelte-ts" | "vite-svelte-js" | "next-ts-latest" | "next-js-latest"
        | "next-ts-16" | "next-js-16" | "next-ts-15" | "next-js-15" | "next-ts-14"
        | "next-js-14" | "cra-ts" | "cra-js" | "angular-ts-latest" | "nestjs-ts-latest"
        | "nestjs-js-latest" | "react-native-ts" | "react-native-js" => vec![
            ToolRequirement::Node,
            ToolRequirement::Npm,
            ToolRequirement::Npx,
        ],
        "laravel-php-latest"
        | "symfony-php-latest"
        | "slim-php-latest"
        | "codeigniter-php-latest" => vec![ToolRequirement::Php, ToolRequirement::Composer],
        "python-basic"
        | "fastapi-python-latest"
        | "flask-python-latest"
        | "django-python-latest" => vec![ToolRequirement::Python],
        "dart-console-latest" => vec![ToolRequirement::Dart],
        "flutter-app-latest" => vec![ToolRequirement::Flutter],
        "go-basic-latest" | "gin-go-latest" | "fiber-go-latest" | "echo-go-latest"
        | "chi-go-latest" => vec![ToolRequirement::Go],
        "java-basic-latest" => vec![ToolRequirement::Java, ToolRequirement::Javac],
        "maven-java-latest" | "spring-boot-java-latest" => {
            vec![
                ToolRequirement::Java,
                ToolRequirement::Javac,
                ToolRequirement::Maven,
            ]
        }
        "gradle-java-latest" => {
            vec![
                ToolRequirement::Java,
                ToolRequirement::Javac,
                ToolRequirement::Gradle,
            ]
        }
        "ruby-basic-latest" => vec![ToolRequirement::Ruby],
        "sinatra-ruby-latest" | "rails-ruby-latest" => {
            vec![ToolRequirement::Ruby, ToolRequirement::Bundler]
        }
        _ => return Err(format!("Unknown project template '{}'", template_id)),
    };

    Ok(requirements)
}

fn prepare_create_toolchain(
    app: &AppHandle,
    state: &AppState,
    creation_id: &str,
    template_id: &str,
) -> Result<CreateToolchain, String> {
    let requirements = template_requirements(template_id)?;
    let mut toolchain = CreateToolchain {
        composer: None,
        python: None,
    };

    if requirements.contains(&ToolRequirement::Node) {
        check_required_tool(
            app,
            creation_id,
            "Node.js",
            node_bin(),
            vec!["--version".to_string()],
        )?;
    }

    if requirements.contains(&ToolRequirement::Npm) {
        check_required_tool(
            app,
            creation_id,
            "npm",
            npm_bin(),
            vec!["--version".to_string()],
        )?;
    }

    if requirements.contains(&ToolRequirement::Npx) {
        check_required_tool(
            app,
            creation_id,
            "npx",
            npx_bin(),
            vec!["--version".to_string()],
        )?;
    }

    if requirements.contains(&ToolRequirement::Php) {
        check_required_tool(
            app,
            creation_id,
            "PHP CLI",
            php_bin(),
            vec!["--version".to_string()],
        )?;
    }

    if requirements.contains(&ToolRequirement::Composer) {
        toolchain.composer = Some(prepare_composer(app, state, creation_id)?);
    }

    if requirements.contains(&ToolRequirement::Python) {
        toolchain.python = Some(prepare_python(app, creation_id)?);
    }

    if requirements.contains(&ToolRequirement::Dart) {
        check_required_tool(
            app,
            creation_id,
            "Dart SDK",
            dart_bin(),
            vec!["--version".to_string()],
        )?;
    }

    if requirements.contains(&ToolRequirement::Flutter) {
        check_required_tool(
            app,
            creation_id,
            "Flutter SDK",
            flutter_bin(),
            vec!["--version".to_string()],
        )?;
    }

    if requirements.contains(&ToolRequirement::Go) {
        check_required_tool(
            app,
            creation_id,
            "Go",
            go_bin(),
            vec!["version".to_string()],
        )?;
    }

    if requirements.contains(&ToolRequirement::Java) {
        check_required_tool(
            app,
            creation_id,
            "Java",
            java_bin(),
            vec!["--version".to_string()],
        )?;
    }

    if requirements.contains(&ToolRequirement::Javac) {
        check_required_tool(
            app,
            creation_id,
            "Java compiler",
            javac_bin(),
            vec!["--version".to_string()],
        )?;
    }

    if requirements.contains(&ToolRequirement::Maven) {
        check_required_tool(
            app,
            creation_id,
            "Maven",
            maven_bin(),
            vec!["--version".to_string()],
        )?;
    }

    if requirements.contains(&ToolRequirement::Gradle) {
        check_required_tool(
            app,
            creation_id,
            "Gradle",
            gradle_bin(),
            vec!["--version".to_string()],
        )?;
    }

    if requirements.contains(&ToolRequirement::Ruby) {
        check_required_tool(
            app,
            creation_id,
            "Ruby",
            ruby_bin(),
            vec!["--version".to_string()],
        )?;
    }

    if requirements.contains(&ToolRequirement::Bundler) {
        check_required_tool(
            app,
            creation_id,
            "Bundler",
            bundle_bin(),
            vec!["--version".to_string()],
        )?;
    }

    emit_create_log(app, creation_id, "Requirements ready.".to_string(), false);
    Ok(toolchain)
}

fn creation_steps(
    template_id: &str,
    project_name: &str,
    parent_dir: &Path,
    target_dir: &Path,
    toolchain: &CreateToolchain,
) -> Result<Vec<CreationStep>, String> {
    let composer = toolchain.composer.as_ref();
    let python = toolchain.python.as_ref();
    let steps = match template_id {
        "python-basic" => python_project_steps(
            "basic",
            project_name,
            parent_dir,
            target_dir,
            python.ok_or_else(|| "Python was not prepared".to_string())?,
        ),
        "dart-console-latest" => dart_console_steps(project_name, parent_dir),
        "flutter-app-latest" => flutter_app_steps(project_name, parent_dir),
        "java-basic-latest" => java_basic_steps(project_name, parent_dir, target_dir),
        "maven-java-latest" => maven_java_steps(project_name, parent_dir, target_dir),
        "gradle-java-latest" => gradle_java_steps(project_name, parent_dir, target_dir),
        "spring-boot-java-latest" => spring_boot_steps(project_name, parent_dir, target_dir),
        "ruby-basic-latest" => ruby_basic_steps(project_name, parent_dir, target_dir),
        "sinatra-ruby-latest" => sinatra_steps(project_name, parent_dir, target_dir),
        "rails-ruby-latest" => rails_steps(project_name, parent_dir, target_dir),
        "go-basic-latest" => go_project_steps("basic", project_name, parent_dir, target_dir),
        "gin-go-latest" => go_project_steps("gin", project_name, parent_dir, target_dir),
        "fiber-go-latest" => go_project_steps("fiber", project_name, parent_dir, target_dir),
        "echo-go-latest" => go_project_steps("echo", project_name, parent_dir, target_dir),
        "chi-go-latest" => go_project_steps("chi", project_name, parent_dir, target_dir),
        "fastapi-python-latest" => python_project_steps(
            "fastapi",
            project_name,
            parent_dir,
            target_dir,
            python.ok_or_else(|| "Python was not prepared".to_string())?,
        ),
        "flask-python-latest" => python_project_steps(
            "flask",
            project_name,
            parent_dir,
            target_dir,
            python.ok_or_else(|| "Python was not prepared".to_string())?,
        ),
        "django-python-latest" => python_project_steps(
            "django",
            project_name,
            parent_dir,
            target_dir,
            python.ok_or_else(|| "Python was not prepared".to_string())?,
        ),
        "node-ts" => node_backend_steps("node", true, project_name, target_dir),
        "node-js" => node_backend_steps("node", false, project_name, target_dir),
        "express-ts" => node_backend_steps("express", true, project_name, target_dir),
        "express-js" => node_backend_steps("express", false, project_name, target_dir),
        "vite-react-ts" => vite_steps("react-ts", project_name, parent_dir, target_dir),
        "vite-react-js" => vite_steps("react", project_name, parent_dir, target_dir),
        "vite-vue-ts" => vite_steps("vue-ts", project_name, parent_dir, target_dir),
        "vite-vue-js" => vite_steps("vue", project_name, parent_dir, target_dir),
        "nuxt-ts-latest" => nuxt_steps(project_name, parent_dir),
        "nuxt-js-latest" => nuxt_steps(project_name, parent_dir),
        "vite-svelte-ts" => vite_steps("svelte-ts", project_name, parent_dir, target_dir),
        "vite-svelte-js" => vite_steps("svelte", project_name, parent_dir, target_dir),
        "laravel-php-latest" => composer_create_project_steps(
            "Create Laravel project",
            "laravel/laravel",
            project_name,
            parent_dir,
            composer.ok_or_else(|| "Composer was not prepared".to_string())?,
        ),
        "symfony-php-latest" => composer_create_project_steps(
            "Create Symfony project",
            "symfony/skeleton",
            project_name,
            parent_dir,
            composer.ok_or_else(|| "Composer was not prepared".to_string())?,
        ),
        "slim-php-latest" => composer_create_project_steps(
            "Create Slim project",
            "slim/slim-skeleton",
            project_name,
            parent_dir,
            composer.ok_or_else(|| "Composer was not prepared".to_string())?,
        ),
        "codeigniter-php-latest" => composer_create_project_steps(
            "Create CodeIgniter project",
            "codeigniter4/appstarter",
            project_name,
            parent_dir,
            composer.ok_or_else(|| "Composer was not prepared".to_string())?,
        ),
        "next-ts-latest" => next_steps("latest", true, project_name, parent_dir),
        "next-js-latest" => next_steps("latest", false, project_name, parent_dir),
        "next-ts-16" => next_steps("16", true, project_name, parent_dir),
        "next-js-16" => next_steps("16", false, project_name, parent_dir),
        "next-ts-15" => next_steps("15", true, project_name, parent_dir),
        "next-js-15" => next_steps("15", false, project_name, parent_dir),
        "next-ts-14" => next_steps("14", true, project_name, parent_dir),
        "next-js-14" => next_steps("14", false, project_name, parent_dir),
        "cra-ts" => cra_steps(true, project_name, parent_dir),
        "cra-js" => cra_steps(false, project_name, parent_dir),
        "nestjs-ts-latest" => nest_steps(true, project_name, parent_dir),
        "nestjs-js-latest" => nest_steps(false, project_name, parent_dir),
        "react-native-ts" => react_native_steps(true, project_name, parent_dir),
        "react-native-js" => react_native_steps(false, project_name, parent_dir),
        "angular-ts-latest" => angular_steps(project_name, parent_dir),
        _ => return Err(format!("Unknown project template '{}'", template_id)),
    };
    Ok(steps)
}

#[tauri::command]
async fn create_project(
    app: AppHandle,
    template_id: String,
    parent_dir: String,
    project_name: String,
    creation_id: String,
) -> Result<ProjectInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        validate_project_name(&project_name)?;

        let parent_path = PathBuf::from(&parent_dir);
        if !parent_path.is_dir() {
            return Err("Parent folder does not exist".to_string());
        }

        let target_path = parent_path.join(&project_name);
        if target_path.exists() {
            return Err("Target folder already exists".to_string());
        }

        let state = app.state::<AppState>();
        let toolchain = prepare_create_toolchain(&app, &state, &creation_id, &template_id)?;
        let steps = creation_steps(
            &template_id,
            &project_name,
            &parent_path,
            &target_path,
            &toolchain,
        )?;
        for step in &steps {
            run_creation_step(&app, &state, &creation_id, step)?;
        }

        emit_create_log(
            &app,
            &creation_id,
            "Project created successfully. Opening workspace...".to_string(),
            false,
        );

        read_package_json(target_path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| format!("Create task failed: {}", e))?
}

#[tauri::command]
fn read_package_json(project_path: String) -> Result<ProjectInfo, String> {
    let project_dir = PathBuf::from(&project_path);
    let composer_path = project_dir.join("composer.json");
    let package_path = project_dir.join("package.json");
    let pyproject_path = project_dir.join("pyproject.toml");
    let main_py_path = project_dir.join("main.py");
    let pubspec_path = project_dir.join("pubspec.yaml");
    let go_mod_path = project_dir.join("go.mod");
    let main_go_path = project_dir.join("main.go");
    let pom_path = project_dir.join("pom.xml");
    let gradle_path = project_dir.join("build.gradle");
    let gradle_kts_path = project_dir.join("build.gradle.kts");
    let gemfile_path = project_dir.join("Gemfile");
    let main_rb_path = project_dir.join("main.rb");

    let mut scripts = Vec::new();

    if composer_path.exists() {
        if let Ok(project) =
            read_composer_project(project_path.clone(), &project_dir, &composer_path)
        {
            scripts.extend(project.scripts);
        }
    }

    if pyproject_path.exists() {
        if let Ok(project) =
            read_python_project(project_path.clone(), &project_dir, &pyproject_path)
        {
            scripts.extend(project.scripts);
        }
    }

    if package_path.exists() {
        if let Ok(project) = read_node_project(project_path.clone(), &project_dir, &package_path) {
            scripts.extend(project.scripts);
        }
    }

    if pubspec_path.exists() {
        if let Ok(project) = read_pub_project(project_path.clone(), &project_dir, &pubspec_path) {
            scripts.extend(project.scripts);
        }
    }

    if go_mod_path.exists() {
        if let Ok(project) = read_go_project(project_path.clone(), &project_dir, &go_mod_path) {
            scripts.extend(project.scripts);
        }
    }

    if pom_path.exists() {
        if let Ok(project) = read_maven_project(project_path.clone(), &project_dir, &pom_path) {
            scripts.extend(project.scripts);
        }
    }

    if gradle_path.exists() || gradle_kts_path.exists() {
        let build_path = if gradle_path.exists() {
            &gradle_path
        } else {
            &gradle_kts_path
        };
        if let Ok(project) = read_gradle_project(project_path.clone(), &project_dir, build_path) {
            scripts.extend(project.scripts);
        }
    }

    if gemfile_path.exists() {
        if let Ok(project) = read_ruby_project(project_path.clone(), &project_dir, &gemfile_path) {
            scripts.extend(project.scripts);
        }
    }

    if main_py_path.exists()
        && !scripts.iter().any(|script: &ScriptInfo| {
            script.package_manager.as_deref() == Some("python") && script.name == "start"
        })
    {
        scripts.push(python_main_script());
    }

    if main_go_path.exists()
        && !scripts.iter().any(|script: &ScriptInfo| {
            script.package_manager.as_deref() == Some("go") && script.name == "start"
        })
    {
        scripts.extend(go_main_scripts(&project_dir));
    }

    if let Some(main_file) = find_java_main_file(&project_dir) {
        if !scripts.iter().any(|script: &ScriptInfo| {
            script.package_manager.as_deref() == Some("java") && script.name == "start"
        }) {
            scripts.extend(java_main_scripts(&main_file));
        }
    }

    if main_rb_path.exists()
        && !scripts.iter().any(|script: &ScriptInfo| {
            script.package_manager.as_deref() == Some("ruby") && script.name == "start"
        })
    {
        scripts.push(ruby_main_script());
    }

    Ok(ProjectInfo {
        name: python_folder_project_name(&project_dir),
        path: project_path,
        scripts,
        package_manager: preferred_package_manager(&project_dir),
    })
}

#[tauri::command]
fn run_script(
    app: AppHandle,
    state: State<'_, AppState>,
    project_path: String,
    script_name: String,
    package_manager: Option<String>,
    run_key: Option<String>,
) -> Result<u32, String> {
    let package_manager = normalize_package_manager(package_manager, &project_path);
    let run_key = run_key.unwrap_or_else(|| format!("{}:{}", package_manager, script_name));
    let process_key = format!("{}::{}", &project_path, &run_key);

    let mut processes = state.running_processes.lock().map_err(|e| e.to_string())?;

    if processes.contains_key(&process_key) {
        return Err(format!("Script '{}' is already running", script_name));
    }

    let mut cmd = script_command(&app, &package_manager, &project_path, &script_name)?;
    cmd.current_dir(&project_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start script: {}", e))?;

    let pid = child.id();
    let run_id = state.next_run_id.fetch_add(1, Ordering::Relaxed);
    processes.insert(process_key.clone(), RunningProcess { run_id, child });
    drop(processes);

    let app_clone = app.clone();
    let key_clone = process_key.clone();
    let project_path_clone = project_path.clone();
    let run_key_clone = run_key.clone();

    std::thread::spawn(move || {
        let (stdout, stderr) = {
            let processes = app_clone.state::<AppState>();
            let mut procs = match processes.running_processes.lock() {
                Ok(p) => p,
                Err(_) => return,
            };
            match procs.get_mut(&key_clone) {
                Some(process) if process.run_id == run_id => {
                    (process.child.stdout.take(), process.child.stderr.take())
                }
                None => return,
                _ => return,
            }
        };

        let app_stdout = app_clone.clone();
        let app_stderr = app_clone.clone();
        let path_stdout = project_path_clone.clone();
        let path_stderr = project_path_clone.clone();
        let name_stdout = run_key_clone.clone();
        let name_stderr = run_key_clone.clone();

        let stdout_thread = stdout.map(|s| {
            std::thread::spawn(move || {
                let reader = BufReader::new(s);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let _ = app_stdout.emit(
                            "script-log",
                            LogEvent {
                                project_path: path_stdout.clone(),
                                script_name: name_stdout.clone(),
                                line,
                                is_error: false,
                            },
                        );
                    }
                }
            })
        });

        let stderr_thread = stderr.map(|s| {
            std::thread::spawn(move || {
                let reader = BufReader::new(s);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let _ = app_stderr.emit(
                            "script-log",
                            LogEvent {
                                project_path: path_stderr.clone(),
                                script_name: name_stderr.clone(),
                                line,
                                is_error: true,
                            },
                        );
                    }
                }
            })
        });

        if let Some(t) = stdout_thread {
            let _ = t.join();
        }
        if let Some(t) = stderr_thread {
            let _ = t.join();
        }

        let finished = {
            let processes = app_clone.state::<AppState>();
            let mut procs = match processes.running_processes.lock() {
                Ok(p) => p,
                Err(_) => return,
            };
            let is_current = procs
                .get(&key_clone)
                .map(|process| process.run_id == run_id)
                .unwrap_or(false);
            if is_current {
                let exit_code = procs
                    .remove(&key_clone)
                    .and_then(|mut process| process.child.wait().ok())
                    .and_then(|s| s.code());
                Some(exit_code)
            } else {
                None
            }
        };

        if let Some(exit_code) = finished {
            let _ = app_clone.emit(
                "process-ended",
                ProcessEndEvent {
                    project_path: project_path_clone,
                    script_name: run_key_clone,
                    exit_code,
                },
            );
        }
    });

    Ok(pid)
}

#[tauri::command]
fn send_script_input(
    state: State<'_, AppState>,
    project_path: String,
    script_name: String,
    input: String,
) -> Result<(), String> {
    let process_key = format!("{}::{}", &project_path, &script_name);
    let mut processes = state.running_processes.lock().map_err(|e| e.to_string())?;
    let process = processes
        .get_mut(&process_key)
        .ok_or_else(|| format!("No running process found for '{}'", script_name))?;

    let stdin = process
        .child
        .stdin
        .as_mut()
        .ok_or_else(|| format!("Process '{}' is not accepting input", script_name))?;

    stdin
        .write_all(input.as_bytes())
        .map_err(|e| format!("Failed to send input: {}", e))?;
    stdin
        .flush()
        .map_err(|e| format!("Failed to flush input: {}", e))
}

#[tauri::command]
fn kill_script(
    state: State<'_, AppState>,
    project_path: String,
    script_name: String,
) -> Result<(), String> {
    let process_key = format!("{}::{}", &project_path, &script_name);
    let process = {
        let mut processes = state.running_processes.lock().map_err(|e| e.to_string())?;
        processes.remove(&process_key)
    };

    let Some(mut process) = process else {
        return Err(format!("No running process found for '{}'", script_name));
    };

    kill_process_group(&mut process.child)
}

#[tauri::command]
fn kill_project_scripts(state: State<'_, AppState>, project_path: String) -> Result<(), String> {
    let process_prefix = format!("{}::", project_path);
    let processes: Vec<RunningProcess> = {
        let mut running_processes = state.running_processes.lock().map_err(|e| e.to_string())?;
        let keys: Vec<String> = running_processes
            .keys()
            .filter(|key| key.starts_with(&process_prefix))
            .cloned()
            .collect();

        keys.into_iter()
            .filter_map(|key| running_processes.remove(&key))
            .collect()
    };

    let mut errors = Vec::new();
    for mut process in processes {
        if let Err(err) = kill_process_group(&mut process.child) {
            errors.push(err);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

#[tauri::command]
fn get_running_scripts(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let processes = state.running_processes.lock().map_err(|e| e.to_string())?;
    Ok(processes.keys().cloned().collect())
}

#[tauri::command]
fn detect_package_managers(project_path: String) -> Result<Vec<String>, String> {
    let mut managers = Vec::new();
    let path = std::path::Path::new(&project_path);

    if path.join("composer.json").exists() {
        managers.push("composer".to_string());
    }

    if path.join("pyproject.toml").exists() {
        managers.push("python".to_string());
    }

    if !managers.iter().any(|manager| manager == "python") && path.join("main.py").exists() {
        managers.push("python".to_string());
    }

    if path.join("pubspec.yaml").exists() {
        managers.push(preferred_pub_package_manager(path));
    }

    if path.join("package.json").exists() {
        managers.push("npm".to_string());
        if path.join("yarn.lock").exists() || path.join(".yarnrc.yml").exists() {
            managers.push("yarn".to_string());
        }
        if path.join("pnpm-lock.yaml").exists() {
            managers.push("pnpm".to_string());
        }
        if path.join("bun.lockb").exists() || path.join("bun.lock").exists() {
            managers.push("bun".to_string());
        }
    }

    Ok(managers)
}

fn pins_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let mut dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    dir.push("pins.json");
    Ok(dir)
}

fn read_all_pins(app: &AppHandle) -> HashMap<String, Vec<String>> {
    let path = match pins_path(app) {
        Ok(p) => p,
        Err(_) => return HashMap::new(),
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };
    serde_json::from_str(&content).unwrap_or_default()
}

fn write_all_pins(app: &AppHandle, pins: &HashMap<String, Vec<String>>) -> Result<(), String> {
    let path = pins_path(app)?;
    let content = serde_json::to_string_pretty(pins).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_pins(app: AppHandle, project_path: String) -> Vec<String> {
    let all = read_all_pins(&app);
    all.get(&project_path).cloned().unwrap_or_default()
}

#[tauri::command]
fn save_pins(app: AppHandle, project_path: String, pins: Vec<String>) -> Result<(), String> {
    let mut all = read_all_pins(&app);
    all.insert(project_path, pins);
    write_all_pins(&app, &all)
}

fn custom_commands_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let mut dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    dir.push("custom_commands.json");
    Ok(dir)
}

fn read_all_custom_commands(app: &AppHandle) -> HashMap<String, Vec<ScriptInfo>> {
    let path = match custom_commands_path(app) {
        Ok(p) => p,
        Err(_) => return HashMap::new(),
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };
    serde_json::from_str(&content).unwrap_or_default()
}

fn write_all_custom_commands(
    app: &AppHandle,
    commands: &HashMap<String, Vec<ScriptInfo>>,
) -> Result<(), String> {
    let path = custom_commands_path(app)?;
    let content = serde_json::to_string_pretty(commands).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

fn custom_script_command(
    app: &AppHandle,
    project_path: &str,
    script_name: &str,
) -> Result<String, String> {
    let all = read_all_custom_commands(app);
    all.get(project_path)
        .and_then(|commands| commands.iter().find(|command| command.name == script_name))
        .map(|command| command.command.clone())
        .filter(|command| !command.trim().is_empty())
        .ok_or_else(|| format!("Custom command '{}' was not found", script_name))
}

#[tauri::command]
fn load_custom_commands(app: AppHandle, project_path: String) -> Vec<ScriptInfo> {
    let all = read_all_custom_commands(&app);
    all.get(&project_path).cloned().unwrap_or_default()
}

#[tauri::command]
fn save_custom_commands(
    app: AppHandle,
    project_path: String,
    commands: Vec<ScriptInfo>,
) -> Result<(), String> {
    let clean_commands: Vec<ScriptInfo> = commands
        .into_iter()
        .filter(|command| !command.name.trim().is_empty() && !command.command.trim().is_empty())
        .map(|command| ScriptInfo {
            name: command.name,
            command: command.command,
            package_manager: Some("custom".to_string()),
            source: Some("Custom".to_string()),
        })
        .collect();
    let mut all = read_all_custom_commands(&app);
    all.insert(project_path, clean_commands);
    write_all_custom_commands(&app, &all)
}

fn recent_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let mut dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    dir.push("recent.json");
    Ok(dir)
}

fn read_recent_projects(app: &AppHandle) -> Vec<String> {
    let path = match recent_path(&app) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    serde_json::from_str(&content).unwrap_or_default()
}

fn write_recent_projects(app: &AppHandle, list: &[String]) -> Result<(), String> {
    let path = recent_path(app)?;
    let content = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_recent_projects(app: AppHandle) -> Vec<String> {
    read_recent_projects(&app)
}

#[tauri::command]
fn save_recent_project(app: AppHandle, project_path: String) -> Result<(), String> {
    let mut list = read_recent_projects(&app);
    list.retain(|p| p != &project_path);
    list.insert(0, project_path);
    if list.len() > 10 {
        list.truncate(10);
    }

    write_recent_projects(&app, &list)
}

#[tauri::command]
fn remove_recent_project(app: AppHandle, project_path: String) -> Result<(), String> {
    let mut list = read_recent_projects(&app);
    list.retain(|p| p != &project_path);
    write_recent_projects(&app, &list)
}

#[tauri::command]
fn clear_recent_projects(app: AppHandle) -> Result<(), String> {
    write_recent_projects(&app, &[])
}

#[tauri::command]
fn open_in_vscode(path: String) -> Result<(), String> {
    Command::new("code")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("Failed to open VS Code: {}", e))?;
    Ok(())
}

#[tauri::command]
fn open_in_terminal(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-a", "Terminal", &path])
            .spawn()
            .map_err(|e| format!("Failed to open Terminal: {}", e))?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", "cmd.exe", "/K", "cd", "/d", &path])
            .spawn()
            .map_err(|e| format!("Failed to open Terminal: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        fn command_exists(name: &str) -> bool {
            Command::new("which")
                .arg(name)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        }

        if command_exists("gnome-terminal") {
            Command::new("gnome-terminal")
                .arg(format!("--working-directory={}", path))
                .spawn()
                .map_err(|e| format!("Failed to open Terminal: {}", e))?;
        } else if command_exists("konsole") {
            Command::new("konsole")
                .args(["--workdir", &path])
                .spawn()
                .map_err(|e| format!("Failed to open Terminal: {}", e))?;
        } else if command_exists("xfce4-terminal") {
            Command::new("xfce4-terminal")
                .args(["--working-directory", &path])
                .spawn()
                .map_err(|e| format!("Failed to open Terminal: {}", e))?;
        } else if command_exists("xterm") {
            Command::new("xterm")
                .args([
                    "-e",
                    "sh",
                    "-lc",
                    "cd \"$1\" && exec \"${SHELL:-sh}\"",
                    "prolaunch-terminal",
                    &path,
                ])
                .spawn()
                .map_err(|e| format!("Failed to open Terminal: {}", e))?;
        } else {
            return Err("No supported terminal found".to_string());
        }
    }
    Ok(())
}

#[tauri::command]
fn list_projects(recent_path: Option<String>) -> Result<Vec<ProjectInfo>, String> {
    let mut projects = Vec::new();

    if let Some(path) = recent_path {
        match read_package_json(path) {
            Ok(info) => projects.push(info),
            Err(_) => {}
        }
    }

    Ok(projects)
}

fn kill_all_processes(app: &AppHandle) {
    let (children, creation_pids): (Vec<Child>, Vec<u32>) = {
        let state = app.state::<AppState>();
        let children = match state.running_processes.lock() {
            Ok(mut procs) => procs.drain().map(|(_, process)| process.child).collect(),
            Err(_) => Vec::new(),
        };
        let creation_pids = match state.creation_processes.lock() {
            Ok(mut procs) => procs.drain().map(|(_, pid)| pid).collect(),
            Err(_) => Vec::new(),
        };
        (children, creation_pids)
    };
    for mut child in children {
        kill_process_group(&mut child).ok();
    }
    for pid in creation_pids {
        kill_process_id(pid).ok();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            running_processes: Mutex::new(HashMap::new()),
            creation_processes: Mutex::new(HashMap::new()),
            next_run_id: AtomicU64::new(1),
        })
        .invoke_handler(tauri::generate_handler![
            create_project,
            read_package_json,
            run_script,
            send_script_input,
            kill_script,
            kill_project_scripts,
            get_running_scripts,
            list_projects,
            detect_package_managers,
            load_pins,
            save_pins,
            load_custom_commands,
            save_custom_commands,
            load_recent_projects,
            save_recent_project,
            remove_recent_project,
            clear_recent_projects,
            open_in_vscode,
            open_in_terminal,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| match event {
        tauri::RunEvent::WindowEvent {
            event: tauri::WindowEvent::CloseRequested { .. },
            ..
        }
        | tauri::RunEvent::ExitRequested { .. }
        | tauri::RunEvent::Exit => {
            kill_all_processes(app_handle);
        }
        _ => {}
    });
}
