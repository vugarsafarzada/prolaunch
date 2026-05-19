use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};
#[cfg(unix)]
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

struct AppState {
    running_processes: Mutex<HashMap<String, RunningProcess>>,
    next_run_id: AtomicU64,
}

struct RunningProcess {
    run_id: u64,
    child: Child,
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
    #[serde(rename = "packageManager")]
    package_manager: String,
}

fn kill_process_group(child: &mut Child) -> Result<(), String> {
    let pid = child.id();

    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .args(["-TERM", &format!("-{}", pid)])
            .status();

        for _ in 0..20 {
            match child.try_wait() {
                Ok(Some(_)) => return Ok(()),
                Ok(None) => std::thread::sleep(Duration::from_millis(100)),
                Err(e) => return Err(format!("wait failed: {}", e)),
            }
        }

        let _ = Command::new("kill")
            .args(["-KILL", &format!("-{}", pid)])
            .status();
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

#[cfg(unix)]
fn parent_watch_script() -> &'static str {
    r#"
parent_pid="$1"
manager="$2"
script_name="$3"

"$manager" run "$script_name" &
child_pid=$!

(
  while kill -0 "$parent_pid" 2>/dev/null; do
    sleep 1
  done
  kill -TERM -$$ 2>/dev/null
  sleep 2
  kill -KILL -$$ 2>/dev/null
) &
watcher_pid=$!

wait "$child_pid"
status=$?
kill "$watcher_pid" 2>/dev/null
wait "$watcher_pid" 2>/dev/null
exit "$status"
"#
}

#[cfg(windows)]
fn parent_watch_script() -> &'static str {
    r#"
$parentId = [int]$args[0]
$manager = $args[1]
$scriptName = $args[2]
$runnerPid = $PID

$watcher = Start-Job -ScriptBlock {
  param($parentId, $runnerPid)
  while (Get-Process -Id $parentId -ErrorAction SilentlyContinue) {
    Start-Sleep -Seconds 1
  }
  & taskkill.exe /F /T /PID $runnerPid | Out-Null
} -ArgumentList $parentId, $runnerPid

try {
  & $manager run $scriptName
  $exitCode = if ($null -eq $LASTEXITCODE) { 0 } else { $LASTEXITCODE }
} finally {
  Stop-Job -Job $watcher -ErrorAction SilentlyContinue | Out-Null
  Remove-Job -Job $watcher -Force -ErrorAction SilentlyContinue | Out-Null
}

exit $exitCode
"#
}

fn script_command(package_manager: &str, script_name: &str) -> Command {
    let manager_command = package_manager_command(package_manager);
    let parent_pid = std::process::id().to_string();

    #[cfg(unix)]
    {
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(parent_watch_script())
            .arg("prolaunch-runner")
            .arg(parent_pid)
            .arg(manager_command)
            .arg(script_name);
        cmd
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
            .arg(parent_watch_script())
            .arg(parent_pid)
            .arg(manager_command)
            .arg(script_name);
        cmd
    }
}

fn preferred_package_manager(path: &std::path::Path) -> String {
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

fn normalize_package_manager(package_manager: Option<String>, project_path: &str) -> String {
    match package_manager.as_deref() {
        Some("npm") | Some("pnpm") | Some("yarn") | Some("bun") => package_manager.unwrap(),
        _ => preferred_package_manager(std::path::Path::new(project_path)),
    }
}

fn package_manager_command(package_manager: &str) -> String {
    if cfg!(target_os = "windows") && package_manager != "bun" {
        format!("{}.cmd", package_manager)
    } else {
        package_manager.to_string()
    }
}

#[tauri::command]
fn read_package_json(project_path: String) -> Result<ProjectInfo, String> {
    let project_dir = std::path::Path::new(&project_path);
    let package_path = project_dir.join("package.json");
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

#[tauri::command]
fn run_script(
    app: AppHandle,
    state: State<'_, AppState>,
    project_path: String,
    script_name: String,
    package_manager: Option<String>,
) -> Result<u32, String> {
    let process_key = format!("{}::{}", &project_path, &script_name);

    let mut processes = state.running_processes.lock().map_err(|e| e.to_string())?;

    if processes.contains_key(&process_key) {
        return Err(format!("Script '{}' is already running", script_name));
    }

    let package_manager = normalize_package_manager(package_manager, &project_path);
    let mut cmd = script_command(&package_manager, &script_name);
    cmd.current_dir(&project_path)
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
    let script_name_clone = script_name.clone();

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
                    script_name: script_name_clone,
                    exit_code,
                },
            );
        }
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
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
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
    let children: Vec<Child> = {
        let state = app.state::<AppState>();
        let result = match state.running_processes.lock() {
            Ok(mut procs) => procs.drain().map(|(_, process)| process.child).collect(),
            Err(_) => return,
        };
        result
    };
    for mut child in children {
        kill_process_group(&mut child).ok();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            running_processes: Mutex::new(HashMap::new()),
            next_run_id: AtomicU64::new(1),
        })
        .invoke_handler(tauri::generate_handler![
            read_package_json,
            run_script,
            kill_script,
            kill_project_scripts,
            get_running_scripts,
            list_projects,
            detect_package_managers,
            load_pins,
            save_pins,
            load_recent_projects,
            save_recent_project,
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
