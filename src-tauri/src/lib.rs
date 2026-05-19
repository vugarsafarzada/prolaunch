use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

struct AppState {
    running_processes: Mutex<HashMap<String, Child>>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct ScriptInfo {
    name: String,
    command: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct LogEvent {
    project_path: String,
    script_name: String,
    line: String,
    is_error: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct ProcessEndEvent {
    project_path: String,
    script_name: String,
    exit_code: Option<i32>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct ProjectInfo {
    name: String,
    path: String,
    scripts: Vec<ScriptInfo>,
}

fn kill_process_group(child: &mut Child) -> Result<(), String> {
    let pid = child.id();

    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .args(["-TERM", &format!("-{}", pid)])
            .spawn();
    }

    #[cfg(windows)]
    {
        let output = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .output()
            .map_err(|e| format!("taskkill failed: {}", e))?;
        if !output.status.success() {
            child.kill().map_err(|e| format!("kill failed: {}", e))?;
        }
    }

    child.wait().ok();
    Ok(())
}

#[tauri::command]
fn read_package_json(project_path: String) -> Result<ProjectInfo, String> {
    let package_path = std::path::Path::new(&project_path).join("package.json");
    let content = std::fs::read_to_string(&package_path)
        .map_err(|e| format!("Failed to read package.json: {}", e))?;

    let parsed: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Invalid package.json: {}", e))?;

    let project_name = parsed
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("Untitled Project")
        .to_string();

    let scripts_obj = parsed
        .get("scripts")
        .and_then(|s| s.as_object())
        .ok_or_else(|| "No scripts found in package.json".to_string())?;

    let scripts: Vec<ScriptInfo> = scripts_obj
        .iter()
        .map(|(name, cmd)| ScriptInfo {
            name: name.clone(),
            command: cmd.as_str().unwrap_or("").to_string(),
        })
        .collect();

    Ok(ProjectInfo {
        name: project_name,
        path: project_path,
        scripts,
    })
}

#[tauri::command]
fn run_script(
    app: AppHandle,
    state: State<'_, AppState>,
    project_path: String,
    script_name: String,
) -> Result<u32, String> {
    let process_key = format!("{}::{}", &project_path, &script_name);

    let mut processes = state.running_processes.lock().map_err(|e| e.to_string())?;

    if processes.contains_key(&process_key) {
        return Err(format!("Script '{}' is already running", script_name));
    }

    let npm_command = if cfg!(target_os = "windows") {
        "npm.cmd"
    } else {
        "npm"
    };

    let mut cmd = Command::new(npm_command);
    cmd.args(["run", &script_name])
        .current_dir(&project_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    let child = cmd.spawn().map_err(|e| format!("Failed to start script: {}", e))?;

    let pid = child.id();
    processes.insert(process_key.clone(), child);
    drop(processes);

    let app_clone = app.clone();
    let key_clone = process_key.clone();
    let project_path_clone = project_path.clone();
    let script_name_clone = script_name.clone();

    std::thread::spawn(move || {
        let (stdout, stderr) = {
            let processes = app_clone.state::<AppState>();
            let mut procs = match processes.running_processes.lock() {
                Ok(p) => p,
                Err(_) => return,
            };
            match procs.get_mut(&key_clone) {
                Some(child) => (child.stdout.take(), child.stderr.take()),
                None => return,
            }
        };

        let app_stdout = app_clone.clone();
        let app_stderr = app_clone.clone();
        let path_stdout = project_path_clone.clone();
        let path_stderr = project_path_clone.clone();
        let name_stdout = script_name_clone.clone();
        let name_stderr = script_name_clone.clone();

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

        let exit_code = {
            let processes = app_clone.state::<AppState>();
            let mut procs = match processes.running_processes.lock() {
                Ok(p) => p,
                Err(_) => return,
            };
            procs
                .remove(&key_clone)
                .and_then(|mut c| c.wait().ok())
                .and_then(|s| s.code())
        };

        let _ = app_clone.emit(
            "process-ended",
            ProcessEndEvent {
                project_path: project_path_clone,
                script_name: script_name_clone,
                exit_code,
            },
        );
    });

    Ok(pid)
}

#[tauri::command]
fn kill_script(
    state: State<'_, AppState>,
    project_path: String,
    script_name: String,
) -> Result<(), String> {
    let process_key = format!("{}::{}", &project_path, &script_name);
    let mut processes = state.running_processes.lock().map_err(|e| e.to_string())?;

    if let Some(mut child) = processes.remove(&process_key) {
        kill_process_group(&mut child)?;
        Ok(())
    } else {
        Err(format!("No running process found for '{}'", script_name))
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

fn recent_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let mut dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    dir.push("recent.json");
    Ok(dir)
}

#[tauri::command]
fn load_recent_projects(app: AppHandle) -> Vec<String> {
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

#[tauri::command]
fn save_recent_project(app: AppHandle, project_path: String) -> Result<(), String> {
    let path = recent_path(&app)?;
    let mut list: Vec<String> = if path.exists() {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    list.retain(|p| p != &project_path);
    list.insert(0, project_path);
    if list.len() > 10 {
        list.truncate(10);
    }

    let content = serde_json::to_string_pretty(&list).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            running_processes: Mutex::new(HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![
            read_package_json,
            run_script,
            kill_script,
            get_running_scripts,
            list_projects,
            detect_package_managers,
            load_pins,
            save_pins,
            load_recent_projects,
            save_recent_project,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
