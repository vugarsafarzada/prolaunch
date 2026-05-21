#![allow(unused_imports)]
//! Shared infrastructure: state, process management, tool helpers, creation steps.

use std::collections::HashMap;
use std::env;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex, OnceLock,
};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

pub(crate) struct AppState {
    pub(crate) running_processes: Mutex<HashMap<String, RunningProcess>>,
    pub(crate) creation_processes: Mutex<HashMap<u64, u32>>,
    pub(crate) next_run_id: AtomicU64,
}

pub(crate) struct RunningProcess {
    pub(crate) run_id: u64,
    pub(crate) child: Child,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub(crate) struct ScriptInfo {
    pub(crate) name: String,
    pub(crate) command: String,
    #[serde(rename = "packageManager", skip_serializing_if = "Option::is_none")]
    pub(crate) package_manager: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) source: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub(crate) struct LogEvent {
    pub(crate) project_path: String,
    pub(crate) script_name: String,
    pub(crate) line: String,
    pub(crate) is_error: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub(crate) struct ProcessEndEvent {
    pub(crate) project_path: String,
    pub(crate) script_name: String,
    pub(crate) exit_code: Option<i32>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub(crate) struct CreateProjectLogEvent {
    pub(crate) creation_id: String,
    pub(crate) line: String,
    pub(crate) is_error: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub(crate) struct ProjectInfo {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) scripts: Vec<ScriptInfo>,
    #[serde(rename = "packageManager")]
    pub(crate) package_manager: String,
}

static EFFECTIVE_PATH: OnceLock<String> = OnceLock::new();

fn push_path_dir(dirs: &mut Vec<PathBuf>, path: PathBuf) {
    if path.is_dir() && !dirs.iter().any(|existing| existing == &path) {
        dirs.push(path);
    }
}

fn push_split_path(dirs: &mut Vec<PathBuf>, value: &str) {
    for path in env::split_paths(value) {
        push_path_dir(dirs, path);
    }
}

fn push_version_manager_bins(dirs: &mut Vec<PathBuf>, base: PathBuf, suffix: &[&str]) {
    let Ok(entries) = std::fs::read_dir(base) else {
        return;
    };

    let mut bins = entries
        .filter_map(Result::ok)
        .map(|entry| {
            suffix
                .iter()
                .fold(entry.path(), |path, segment| path.join(segment))
        })
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();

    bins.sort();
    bins.reverse();

    for bin in bins {
        push_path_dir(dirs, bin);
    }
}

fn append_developer_path_dirs(dirs: &mut Vec<PathBuf>) {
    for path in [
        "/opt/homebrew/bin",
        "/opt/homebrew/sbin",
        "/opt/homebrew/opt/openjdk/bin",
        "/opt/homebrew/opt/maven/bin",
        "/opt/homebrew/opt/gradle/bin",
        "/usr/local/bin",
        "/usr/local/sbin",
        "/usr/local/opt/openjdk/bin",
        "/usr/local/opt/maven/bin",
        "/usr/local/opt/gradle/bin",
        "/usr/local/go/bin",
        "/usr/bin",
        "/bin",
        "/usr/sbin",
        "/sbin",
        "/snap/bin",
    ] {
        push_path_dir(dirs, PathBuf::from(path));
    }

    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return;
    };

    for path in [
        ".local/bin",
        ".cargo/bin",
        ".volta/bin",
        ".bun/bin",
        ".deno/bin",
        ".asdf/shims",
        ".pyenv/shims",
        ".rbenv/shims",
        "go/bin",
        ".sdkman/candidates/java/current/bin",
        ".sdkman/candidates/maven/current/bin",
        ".sdkman/candidates/gradle/current/bin",
    ] {
        push_path_dir(dirs, home.join(path));
    }

    push_version_manager_bins(dirs, home.join(".nvm/versions/node"), &["bin"]);
    push_version_manager_bins(
        dirs,
        home.join(".fnm/node-versions"),
        &["installation", "bin"],
    );
    push_version_manager_bins(
        dirs,
        home.join(".local/share/fnm/node-versions"),
        &["installation", "bin"],
    );
}

pub(crate) fn effective_path() -> &'static str {
    EFFECTIVE_PATH.get_or_init(|| {
        let mut dirs = Vec::new();
        if let Ok(path) = env::var("PATH") {
            push_split_path(&mut dirs, &path);
        }
        append_developer_path_dirs(&mut dirs);

        env::join_paths(dirs)
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|_| env::var("PATH").unwrap_or_default())
    })
}

pub(crate) fn apply_command_environment(command: &mut Command) {
    command.env("PATH", effective_path());
}

pub(crate) fn resolve_program(program: &str) -> String {
    let program_path = Path::new(program);
    if program_path.is_absolute() || program_path.components().count() > 1 {
        return program.to_string();
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        for dir in env::split_paths(effective_path()) {
            let candidate = dir.join(program);
            let Ok(metadata) = candidate.metadata() else {
                continue;
            };
            if metadata.is_file() && metadata.permissions().mode() & 0o111 != 0 {
                return candidate.to_string_lossy().to_string();
            }
        }
    }

    program.to_string()
}

#[cfg(unix)]
fn unix_process_group(pid: u32) -> Option<libc::pid_t> {
    let pgid = unsafe { libc::getpgid(pid as libc::pid_t) };
    (pgid > 0).then_some(pgid)
}

#[cfg(unix)]
fn unix_signal_process(pid: libc::pid_t, signal: libc::c_int) -> Result<(), String> {
    let result = unsafe { libc::kill(pid, signal) };
    if result == 0 {
        return Ok(());
    }

    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        return Ok(());
    }

    Err(error.to_string())
}

#[cfg(unix)]
fn unix_signal_owned_group(pid: u32, signal: libc::c_int) -> Result<(), String> {
    let pgid = unix_process_group(pid);
    if pgid == Some(pid as libc::pid_t) {
        unix_signal_process(-(pid as libc::pid_t), signal)
    } else {
        unix_signal_process(pid as libc::pid_t, signal)
    }
}

pub(crate) fn isolate_child_process(command: &mut Command) {
    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;

        command.pre_exec(|| {
            if libc::setsid() == -1 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
}

pub(crate) fn kill_process_id(pid: u32) -> Result<(), String> {
    #[cfg(unix)]
    {
        let _ = unix_signal_owned_group(pid, libc::SIGTERM);
        std::thread::sleep(Duration::from_secs(2));
        let _ = unix_signal_owned_group(pid, libc::SIGKILL);
    }

    #[cfg(windows)]
    {
        let output = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .output()
            .map_err(|e| format!("taskkill failed: {}", e))?;
        if !output.status.success() {
            return Err(format!("taskkill failed for PID {}", pid));
        }
    }

    Ok(())
}

pub(crate) fn kill_process_group(child: &mut Child) -> Result<(), String> {
    let pid = child.id();

    #[cfg(unix)]
    {
        let _ = unix_signal_owned_group(pid, libc::SIGTERM);

        for _ in 0..20 {
            match child.try_wait() {
                Ok(Some(_)) => return Ok(()),
                Ok(None) => std::thread::sleep(Duration::from_millis(100)),
                Err(e) => return Err(format!("wait failed: {}", e)),
            }
        }

        let _ = unix_signal_owned_group(pid, libc::SIGKILL);
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
pub(crate) fn creation_parent_watch_script() -> &'static str {
    r#"
parent_pid="$1"
shift
runner_pid=$$

stop_runner() {
  runner_pgid="$(ps -o pgid= -p "$runner_pid" 2>/dev/null | tr -d '[:space:]')"
  if [ "$runner_pgid" = "$runner_pid" ]; then
    kill -TERM -"$runner_pid" 2>/dev/null
    sleep 2
    kill -KILL -"$runner_pid" 2>/dev/null
  else
    kill -TERM "$runner_pid" 2>/dev/null
    sleep 2
    kill -KILL "$runner_pid" 2>/dev/null
  fi
}

(
  while kill -0 "$parent_pid" 2>/dev/null && kill -0 "$runner_pid" 2>/dev/null; do
    sleep 1
  done
  stop_runner
) >/dev/null 2>&1 </dev/null &

exec "$@"
"#
}

#[cfg(windows)]
pub(crate) fn creation_parent_watch_script() -> &'static str {
    r#"
$parentId = [int]$args[0]
$program = $args[1]
$programArgs = @()
if ($args.Length -gt 2) {
  $programArgs = $args[2..($args.Length - 1)]
}
$runnerPid = $PID

$watcher = Start-Job -ScriptBlock {
  param($parentId, $runnerPid)
  while (Get-Process -Id $parentId -ErrorAction SilentlyContinue) {
    Start-Sleep -Seconds 1
  }
  & taskkill.exe /F /T /PID $runnerPid | Out-Null
} -ArgumentList $parentId, $runnerPid

try {
  & $program @programArgs
  $exitCode = if ($null -eq $LASTEXITCODE) { 0 } else { $LASTEXITCODE }
} finally {
  Stop-Job -Job $watcher -ErrorAction SilentlyContinue | Out-Null
  Remove-Job -Job $watcher -Force -ErrorAction SilentlyContinue | Out-Null
}

exit $exitCode
"#
}

pub(crate) struct CreationStep {
    pub(crate) label: String,
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) cwd: PathBuf,
    pub(crate) display_command: Option<String>,
    pub(crate) action: CreationStepAction,
}

pub(crate) enum CreationStepAction {
    Command,
    WriteFiles(Vec<(PathBuf, String)>),
}

#[derive(Clone)]
pub(crate) struct ToolCommand {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
}

pub(crate) struct CreateToolchain {
    pub(crate) composer: Option<ToolCommand>,
    pub(crate) python: Option<ToolCommand>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolRequirement {
    Node,
    Npm,
    Npx,
    Php,
    Composer,
    Python,
    Dart,
    Flutter,
    Go,
    Java,
    Javac,
    Maven,
    Gradle,
    Ruby,
    Bundler,
}

pub(crate) fn tool_args(tool: &ToolCommand, extra: &[&str]) -> Vec<String> {
    let mut args = tool.args.clone();
    args.extend(extra.iter().map(|arg| arg.to_string()));
    args
}

pub(crate) fn tool_args_owned(tool: &ToolCommand, extra: Vec<String>) -> Vec<String> {
    let mut args = tool.args.clone();
    args.extend(extra);
    args
}

pub(crate) fn shell_quote(value: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("\"{}\"", value.replace('"', "`\""))
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

pub(crate) fn tool_command_line(tool: &ToolCommand, extra: &[String]) -> String {
    std::iter::once(shell_quote(&tool.program))
        .chain(tool.args.iter().map(|arg| shell_quote(arg)))
        .chain(extra.iter().map(|arg| shell_quote(arg)))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn command_output(
    program: &str,
    args: &[String],
    cwd: Option<&Path>,
) -> Result<String, String> {
    let resolved_program = resolve_program(program);
    let mut command = Command::new(&resolved_program);
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    apply_command_environment(&mut command);

    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }

    let output = command
        .output()
        .map_err(|e| format!("{} failed to start: {}", program, e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(command_output_excerpt(&stdout, &stderr))
    } else {
        let details = command_output_excerpt(&stdout, &stderr);
        Err(if details.is_empty() {
            format!("{} exited with status {}", program, output.status)
        } else {
            details
        })
    }
}

pub(crate) fn first_output_line(output: &str) -> String {
    output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("ready")
        .to_string()
}

pub(crate) fn check_required_tool(
    app: &AppHandle,
    creation_id: &str,
    label: &str,
    program: String,
    version_args: Vec<String>,
) -> Result<String, String> {
    emit_create_log(app, creation_id, format!("Checking {}...", label), false);

    match command_output(&program, &version_args, None) {
        Ok(output) => {
            let version = first_output_line(&output);
            emit_create_log(
                app,
                creation_id,
                format!("{} found: {}", label, version),
                false,
            );
            Ok(version)
        }
        Err(details) => Err(format!(
            "{} is required for this template but was not found or could not run.\n{}",
            label, details
        )),
    }
}

pub(crate) fn command_display(tool: &ToolCommand) -> String {
    Path::new(&tool.program)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| tool.program.clone())
}

pub(crate) fn shell_tool_command(command_line: String) -> ToolCommand {
    if cfg!(target_os = "windows") {
        ToolCommand {
            program: "powershell.exe".to_string(),
            args: vec![
                "-NoLogo".to_string(),
                "-NoProfile".to_string(),
                "-NonInteractive".to_string(),
                "-ExecutionPolicy".to_string(),
                "Bypass".to_string(),
                "-Command".to_string(),
                command_line,
            ],
        }
    } else {
        ToolCommand {
            program: "sh".to_string(),
            args: vec!["-c".to_string(), command_line],
        }
    }
}

pub(crate) fn package_manager_command(package_manager: &str) -> String {
    if cfg!(target_os = "windows") {
        return match package_manager {
            "bun" => "bun".to_string(),
            "composer" => "composer.bat".to_string(),
            "go" => "go.exe".to_string(),
            "java" => "java.exe".to_string(),
            "maven" => "mvn.cmd".to_string(),
            "gradle" => "gradle.bat".to_string(),
            "ruby" => "ruby.exe".to_string(),
            _ => format!("{}.cmd", package_manager),
        };
    }

    package_manager.to_string()
}

pub(crate) fn package_manager_run_command(package_manager: &str) -> &'static str {
    if package_manager == "composer" {
        "run-script"
    } else {
        "run"
    }
}

pub(crate) fn creation_step(
    label: &str,
    program: String,
    args: Vec<&str>,
    cwd: &Path,
) -> CreationStep {
    CreationStep {
        label: label.to_string(),
        program,
        args: args.into_iter().map(str::to_string).collect(),
        cwd: cwd.to_path_buf(),
        display_command: None,
        action: CreationStepAction::Command,
    }
}

pub(crate) fn creation_step_from_strings(
    label: &str,
    program: String,
    args: Vec<String>,
    cwd: &Path,
) -> CreationStep {
    CreationStep {
        label: label.to_string(),
        program,
        args,
        cwd: cwd.to_path_buf(),
        display_command: None,
        action: CreationStepAction::Command,
    }
}

pub(crate) fn creation_step_with_display(
    label: &str,
    program: String,
    args: Vec<String>,
    cwd: &Path,
    display_command: String,
) -> CreationStep {
    CreationStep {
        label: label.to_string(),
        program,
        args,
        cwd: cwd.to_path_buf(),
        display_command: Some(display_command),
        action: CreationStepAction::Command,
    }
}

pub(crate) fn write_files_step(
    label: &str,
    files: Vec<(PathBuf, String)>,
    cwd: &Path,
    display_command: String,
) -> CreationStep {
    CreationStep {
        label: label.to_string(),
        program: String::new(),
        args: Vec::new(),
        cwd: cwd.to_path_buf(),
        display_command: Some(display_command),
        action: CreationStepAction::WriteFiles(files),
    }
}

pub(crate) fn creation_command(step: &CreationStep) -> Command {
    let parent_pid = std::process::id().to_string();

    #[cfg(unix)]
    {
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .arg(creation_parent_watch_script())
            .arg("prolaunch-create-step")
            .arg(parent_pid)
            .arg(&step.program)
            .args(&step.args);
        command
    }

    #[cfg(windows)]
    {
        let mut command = Command::new("powershell.exe");
        command
            .arg("-NoLogo")
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-Command")
            .arg(creation_parent_watch_script())
            .arg(parent_pid)
            .arg(&step.program)
            .args(&step.args);
        command
    }
}

pub(crate) fn sanitize_log_line(line: &str) -> String {
    let mut output = String::new();
    let mut chars = line.chars();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }

        if ch != '\u{8}' {
            output.push(ch);
        }
    }

    output.trim().to_string()
}

pub(crate) fn next_line_break(text: &str) -> Option<usize> {
    match (text.find('\n'), text.find('\r')) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

pub(crate) fn emit_create_log(app: &AppHandle, creation_id: &str, line: String, is_error: bool) {
    let line = sanitize_log_line(&line);
    if line.is_empty() {
        return;
    }

    let _ = app.emit(
        "project-create-log",
        CreateProjectLogEvent {
            creation_id: creation_id.to_string(),
            line,
            is_error,
        },
    );
}

pub(crate) fn spawn_create_output_reader<R>(
    app: AppHandle,
    creation_id: String,
    mut stream: R,
    is_error: bool,
) -> std::thread::JoinHandle<String>
where
    R: Read + Send + 'static,
{
    std::thread::spawn(move || {
        let mut output = String::new();
        let mut pending = String::new();
        let mut buffer = [0; 4096];

        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    let chunk = String::from_utf8_lossy(&buffer[..n]);
                    output.push_str(&chunk);
                    pending.push_str(&chunk);

                    while let Some(index) = next_line_break(&pending) {
                        let line = pending[..index].to_string();
                        pending = pending[index + 1..].to_string();
                        emit_create_log(&app, &creation_id, line, is_error);
                    }
                }
                Err(_) => break,
            }
        }

        if !pending.is_empty() {
            emit_create_log(&app, &creation_id, pending, is_error);
        }

        output
    })
}

pub(crate) fn join_reader(handle: Option<std::thread::JoinHandle<String>>) -> String {
    handle
        .and_then(|thread| thread.join().ok())
        .unwrap_or_default()
}

pub(crate) fn command_output_excerpt(stdout: &str, stderr: &str) -> String {
    let mut output = String::new();
    if !stdout.trim().is_empty() {
        output.push_str(stdout.trim());
    }
    if !stderr.trim().is_empty() {
        if !output.is_empty() {
            output.push_str("\n\n");
        }
        output.push_str(stderr.trim());
    }

    const MAX_LEN: usize = 4000;
    if output.chars().count() > MAX_LEN {
        let tail: String = output
            .chars()
            .rev()
            .take(MAX_LEN)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("...{}", tail)
    } else {
        output
    }
}

pub(crate) fn run_creation_step(
    app: &AppHandle,
    state: &AppState,
    creation_id: &str,
    step: &CreationStep,
) -> Result<(), String> {
    emit_create_log(app, creation_id, format!("{}...", step.label), false);
    let display_command = step
        .display_command
        .clone()
        .unwrap_or_else(|| format!("$ {} {}", step.program, step.args.join(" ")));
    emit_create_log(app, creation_id, display_command, false);

    if let CreationStepAction::WriteFiles(files) = &step.action {
        for (path, content) in files {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("{} failed: {}", step.label, e))?;
            }
            std::fs::write(path, content).map_err(|e| format!("{} failed: {}", step.label, e))?;
        }

        emit_create_log(app, creation_id, format!("{} completed", step.label), false);
        return Ok(());
    }

    let mut command = creation_command(step);
    command
        .current_dir(&step.cwd)
        .env("CI", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    apply_command_environment(&mut command);

    isolate_child_process(&mut command);

    let mut child = command
        .spawn()
        .map_err(|e| format!("{} failed to start: {}", step.label, e))?;
    let creation_process_id = state.next_run_id.fetch_add(1, Ordering::Relaxed);
    let creation_pid = child.id();
    {
        let mut processes = state.creation_processes.lock().map_err(|e| e.to_string())?;
        processes.insert(creation_process_id, creation_pid);
    }

    let stdout = child.stdout.take().map(|stream| {
        spawn_create_output_reader(app.clone(), creation_id.to_string(), stream, false)
    });
    let stderr = child.stderr.take().map(|stream| {
        spawn_create_output_reader(app.clone(), creation_id.to_string(), stream, true)
    });

    let started_at = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if started_at.elapsed() > Duration::from_secs(600) {
                    let _ = kill_process_group(&mut child);
                    if let Ok(mut processes) = state.creation_processes.lock() {
                        processes.remove(&creation_process_id);
                    }
                    let stdout = join_reader(stdout);
                    let stderr = join_reader(stderr);
                    let output = command_output_excerpt(&stdout, &stderr);
                    return Err(format!(
                        "{} timed out after 10 minutes\n{}",
                        step.label, output
                    ));
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(e) => {
                let _ = kill_process_group(&mut child);
                if let Ok(mut processes) = state.creation_processes.lock() {
                    processes.remove(&creation_process_id);
                }
                return Err(format!("{} failed while waiting: {}", step.label, e));
            }
        }
    };

    if let Ok(mut processes) = state.creation_processes.lock() {
        processes.remove(&creation_process_id);
    }

    let stdout = join_reader(stdout);
    let stderr = join_reader(stderr);

    if status.success() {
        emit_create_log(app, creation_id, format!("{} completed", step.label), false);
        return Ok(());
    }

    let output = command_output_excerpt(&stdout, &stderr);
    let code = status
        .code()
        .map(|code| code.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    Err(format!(
        "{} failed with exit code {}\n{}",
        step.label, code, output
    ))
}

pub(crate) fn validate_project_name(project_name: &str) -> Result<(), String> {
    let name = project_name.trim();
    if name.is_empty() {
        return Err("Project name is required".to_string());
    }
    if name != project_name {
        return Err("Project name cannot start or end with whitespace".to_string());
    }
    if matches!(name, "." | "..") {
        return Err("Project name is not valid".to_string());
    }
    if name
        .chars()
        .any(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_')))
    {
        return Err(
            "Project name can only use lowercase letters, numbers, dots, dashes, or underscores"
                .to_string(),
        );
    }
    if !name
        .chars()
        .next()
        .map(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        .unwrap_or(false)
    {
        return Err("Project name must start with a lowercase letter or number".to_string());
    }
    Ok(())
}

pub(crate) fn python_folder_project_name(project_dir: &Path) -> String {
    project_dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| "Untitled Project".to_string())
}
