use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};

struct AppState {
    running_processes: Mutex<HashMap<String, RunningProcess>>,
    creation_processes: Mutex<HashMap<u64, u32>>,
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
struct CreateProjectLogEvent {
    creation_id: String,
    line: String,
    is_error: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct ProjectInfo {
    name: String,
    path: String,
    scripts: Vec<ScriptInfo>,
    #[serde(rename = "packageManager")]
    package_manager: String,
}

fn kill_process_id(pid: u32) -> Result<(), String> {
    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .args(["-TERM", &format!("-{}", pid)])
            .status();
        std::thread::sleep(Duration::from_secs(2));
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
            return Err(format!("taskkill failed for PID {}", pid));
        }
    }

    Ok(())
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
fn creation_parent_watch_script() -> &'static str {
    r#"
parent_pid="$1"
shift

"$@" &
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
fn creation_parent_watch_script() -> &'static str {
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

fn script_invocation(
    app: &AppHandle,
    package_manager: &str,
    script_name: &str,
) -> Result<ToolCommand, String> {
    let run_command = package_manager_run_command(package_manager).to_string();

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

    Ok(ToolCommand {
        program: package_manager_command(package_manager),
        args: vec![run_command, script_name.to_string()],
    })
}

fn script_command(
    app: &AppHandle,
    package_manager: &str,
    script_name: &str,
) -> Result<Command, String> {
    let invocation = script_invocation(app, package_manager, script_name)?;
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

fn preferred_package_manager(path: &std::path::Path) -> String {
    if path.join("composer.json").exists() {
        "composer".to_string()
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
        Some("npm") | Some("pnpm") | Some("yarn") | Some("bun") | Some("composer") => {
            package_manager.unwrap()
        }
        _ => preferred_package_manager(std::path::Path::new(project_path)),
    }
}

fn package_manager_command(package_manager: &str) -> String {
    if cfg!(target_os = "windows") {
        return match package_manager {
            "bun" => "bun".to_string(),
            "composer" => "composer.bat".to_string(),
            _ => format!("{}.cmd", package_manager),
        };
    }

    package_manager.to_string()
}

fn package_manager_run_command(package_manager: &str) -> &'static str {
    if package_manager == "composer" {
        "run-script"
    } else {
        "run"
    }
}

struct CreationStep {
    label: String,
    program: String,
    args: Vec<String>,
    cwd: PathBuf,
    display_command: Option<String>,
}

#[derive(Clone)]
struct ToolCommand {
    program: String,
    args: Vec<String>,
}

struct CreateToolchain {
    composer: Option<ToolCommand>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ToolRequirement {
    Node,
    Npm,
    Npx,
    Php,
    Composer,
}

fn npm_bin() -> String {
    if cfg!(target_os = "windows") {
        "npm.cmd".to_string()
    } else {
        "npm".to_string()
    }
}

fn npx_bin() -> String {
    if cfg!(target_os = "windows") {
        "npx.cmd".to_string()
    } else {
        "npx".to_string()
    }
}

fn composer_bin() -> String {
    package_manager_command("composer")
}

fn php_bin() -> String {
    "php".to_string()
}

fn node_bin() -> String {
    "node".to_string()
}

fn command_output(program: &str, args: &[String], cwd: Option<&Path>) -> Result<String, String> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

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

fn first_output_line(output: &str) -> String {
    output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("ready")
        .to_string()
}

fn check_required_tool(
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

fn app_managed_composer_path(app: &AppHandle) -> Result<PathBuf, String> {
    let mut dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    dir.push("tools");
    dir.push("composer");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to prepare tool cache: {}", e))?;
    Ok(dir.join("composer.phar"))
}

fn managed_composer_command(composer_path: &Path) -> ToolCommand {
    ToolCommand {
        program: php_bin(),
        args: vec![composer_path.to_string_lossy().to_string()],
    }
}

fn verify_managed_composer(app: &AppHandle, creation_id: &str, composer_path: &Path) -> bool {
    if !composer_path.exists() {
        return false;
    }

    let args = vec![
        composer_path.to_string_lossy().to_string(),
        "--version".to_string(),
    ];

    match command_output(&php_bin(), &args, None) {
        Ok(output) => {
            emit_create_log(
                app,
                creation_id,
                format!(
                    "Using ProLaunch-managed Composer: {}",
                    first_output_line(&output)
                ),
                false,
            );
            true
        }
        Err(_) => false,
    }
}

fn composer_install_steps(composer_path: &Path) -> Result<Vec<CreationStep>, String> {
    let cache_dir = composer_path
        .parent()
        .ok_or_else(|| "Failed to resolve Composer cache folder".to_string())?;
    let install_dir = cache_dir.to_string_lossy().to_string();
    let composer_file = composer_path.to_string_lossy().to_string();

    let download_installer = r#"
$ok = copy('https://getcomposer.org/installer', 'composer-setup.php');
if (!$ok) {
    fwrite(STDERR, "Could not download Composer installer\n");
    exit(1);
}
echo "Composer installer downloaded\n";
"#;

    let verify_installer = r#"
$expected = trim(file_get_contents('https://composer.github.io/installer.sig'));
$actual = hash_file('sha384', 'composer-setup.php');
if (!$expected || !hash_equals($expected, $actual)) {
    @unlink('composer-setup.php');
    fwrite(STDERR, "Invalid Composer installer checksum\n");
    exit(1);
}
echo "Composer installer verified\n";
"#;

    Ok(vec![
        creation_step_with_display(
            "Download Composer installer",
            php_bin(),
            vec!["-r".to_string(), download_installer.to_string()],
            cache_dir,
            "$ php -r \"download Composer installer\"".to_string(),
        ),
        creation_step_with_display(
            "Verify Composer installer",
            php_bin(),
            vec!["-r".to_string(), verify_installer.to_string()],
            cache_dir,
            "$ php -r \"verify Composer installer\"".to_string(),
        ),
        creation_step_with_display(
            "Install Composer for ProLaunch",
            php_bin(),
            vec![
                "composer-setup.php".to_string(),
                "--quiet".to_string(),
                "--install-dir".to_string(),
                install_dir,
                "--filename".to_string(),
                "composer.phar".to_string(),
            ],
            cache_dir,
            format!(
                "$ php composer-setup.php --install-dir <ProLaunch tools> --filename composer.phar"
            ),
        ),
        creation_step_with_display(
            "Verify Composer",
            php_bin(),
            vec![composer_file, "--version".to_string()],
            cache_dir,
            "$ php <ProLaunch tools>/composer.phar --version".to_string(),
        ),
    ])
}

fn prepare_composer(
    app: &AppHandle,
    state: &AppState,
    creation_id: &str,
) -> Result<ToolCommand, String> {
    emit_create_log(app, creation_id, "Checking Composer...".to_string(), false);
    let system_composer = composer_bin();
    let version_args = vec!["--version".to_string()];

    if let Ok(output) = command_output(&system_composer, &version_args, None) {
        emit_create_log(
            app,
            creation_id,
            format!("Composer found: {}", first_output_line(&output)),
            false,
        );
        return Ok(ToolCommand {
            program: system_composer,
            args: Vec::new(),
        });
    }

    emit_create_log(
        app,
        creation_id,
        "Composer not found on PATH. Preparing ProLaunch-managed Composer...".to_string(),
        false,
    );

    let composer_path = app_managed_composer_path(app)?;
    if verify_managed_composer(app, creation_id, &composer_path) {
        return Ok(managed_composer_command(&composer_path));
    }

    if composer_path.exists() {
        let _ = std::fs::remove_file(&composer_path);
    }

    let install_steps = composer_install_steps(&composer_path)?;
    for step in &install_steps {
        run_creation_step(app, state, creation_id, step)?;
    }

    let installer_path = composer_path
        .parent()
        .map(|dir| dir.join("composer-setup.php"));
    if let Some(installer_path) = installer_path {
        let _ = std::fs::remove_file(installer_path);
    }

    emit_create_log(
        app,
        creation_id,
        "Composer installed for ProLaunch and will be reused next time.".to_string(),
        false,
    );

    Ok(managed_composer_command(&composer_path))
}

fn template_requirements(template_id: &str) -> Result<Vec<ToolRequirement>, String> {
    let requirements = match template_id {
        "vite-react-ts" | "vite-react-js" | "vite-vue-ts" | "vite-vue-js" | "nuxt-ts-latest"
        | "nuxt-js-latest" | "vite-svelte-ts" | "vite-svelte-js" | "next-ts-latest"
        | "next-js-latest" | "next-ts-16" | "next-js-16" | "next-ts-15" | "next-js-15"
        | "next-ts-14" | "next-js-14" | "cra-ts" | "cra-js" | "angular-ts-latest" => {
            vec![
                ToolRequirement::Node,
                ToolRequirement::Npm,
                ToolRequirement::Npx,
            ]
        }
        "laravel-php-latest"
        | "symfony-php-latest"
        | "slim-php-latest"
        | "codeigniter-php-latest" => vec![ToolRequirement::Php, ToolRequirement::Composer],
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
    let mut toolchain = CreateToolchain { composer: None };

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

    emit_create_log(app, creation_id, "Requirements ready.".to_string(), false);
    Ok(toolchain)
}

fn validate_project_name(project_name: &str) -> Result<(), String> {
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

fn creation_step(label: &str, program: String, args: Vec<&str>, cwd: &Path) -> CreationStep {
    CreationStep {
        label: label.to_string(),
        program,
        args: args.into_iter().map(str::to_string).collect(),
        cwd: cwd.to_path_buf(),
        display_command: None,
    }
}

fn creation_step_from_strings(
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
    }
}

fn creation_step_with_display(
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
    }
}

fn vite_steps(
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

fn next_steps(
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

fn cra_steps(use_typescript: bool, project_name: &str, parent_dir: &Path) -> Vec<CreationStep> {
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

fn nuxt_steps(project_name: &str, parent_dir: &Path) -> Vec<CreationStep> {
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

fn composer_create_project_steps(
    label: &str,
    package: &str,
    project_name: &str,
    parent_dir: &Path,
    composer: &ToolCommand,
) -> Vec<CreationStep> {
    let mut args = composer.args.clone();
    args.extend([
        "create-project".to_string(),
        "--no-interaction".to_string(),
        "--no-progress".to_string(),
        package.to_string(),
        project_name.to_string(),
    ]);

    vec![creation_step_from_strings(
        label,
        composer.program.clone(),
        args,
        parent_dir,
    )]
}

fn angular_steps(project_name: &str, parent_dir: &Path) -> Vec<CreationStep> {
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

fn creation_steps(
    template_id: &str,
    project_name: &str,
    parent_dir: &Path,
    target_dir: &Path,
    toolchain: &CreateToolchain,
) -> Result<Vec<CreationStep>, String> {
    let composer = toolchain.composer.as_ref();
    let steps = match template_id {
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
        "angular-ts-latest" => angular_steps(project_name, parent_dir),
        _ => return Err(format!("Unknown project template '{}'", template_id)),
    };
    Ok(steps)
}

fn creation_command(step: &CreationStep) -> Command {
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

fn sanitize_log_line(line: &str) -> String {
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

fn next_line_break(text: &str) -> Option<usize> {
    match (text.find('\n'), text.find('\r')) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn emit_create_log(app: &AppHandle, creation_id: &str, line: String, is_error: bool) {
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

fn spawn_create_output_reader<R>(
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

fn join_reader(handle: Option<std::thread::JoinHandle<String>>) -> String {
    handle
        .and_then(|thread| thread.join().ok())
        .unwrap_or_default()
}

fn command_output_excerpt(stdout: &str, stderr: &str) -> String {
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

fn run_creation_step(
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

    let mut command = creation_command(step);
    command
        .current_dir(&step.cwd)
        .env("CI", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

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

    if composer_path.exists() {
        return read_composer_project(project_path, &project_dir, &composer_path);
    }

    if package_path.exists() {
        return read_node_project(project_path, &project_dir, &package_path);
    }

    Err("No package.json or composer.json found in selected folder".to_string())
}

fn read_node_project(
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

fn composer_script_command(cmd: &serde_json::Value) -> String {
    if let Some(command) = cmd.as_str() {
        return command.to_string();
    }

    if let Some(commands) = cmd.as_array() {
        return commands
            .iter()
            .filter_map(|item| item.as_str())
            .collect::<Vec<_>>()
            .join(" && ");
    }

    String::new()
}

fn is_composer_lifecycle_script(name: &str) -> bool {
    name == "auto-scripts" || name.starts_with("pre-") || name.starts_with("post-")
}

fn composer_project_name(project_dir: &Path, parsed: &serde_json::Value) -> String {
    project_dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .or_else(|| {
            parsed
                .get("name")
                .and_then(|name| name.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "Untitled Project".to_string())
}

fn read_composer_project(
    project_path: String,
    project_dir: &Path,
    composer_path: &Path,
) -> Result<ProjectInfo, String> {
    let content = std::fs::read_to_string(composer_path)
        .map_err(|e| format!("Failed to read composer.json: {}", e))?;

    let parsed: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Invalid composer.json: {}", e))?;

    let project_name = composer_project_name(project_dir, &parsed);

    let scripts: Vec<ScriptInfo> = parsed
        .get("scripts")
        .and_then(|scripts| scripts.as_object())
        .map(|scripts_obj| {
            scripts_obj
                .iter()
                .filter(|(name, _)| !is_composer_lifecycle_script(name))
                .map(|(name, cmd)| ScriptInfo {
                    name: name.clone(),
                    command: composer_script_command(cmd),
                })
                .filter(|script| !script.command.is_empty())
                .collect()
        })
        .unwrap_or_default();

    Ok(ProjectInfo {
        name: project_name,
        path: project_path,
        scripts,
        package_manager: "composer".to_string(),
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
    let mut cmd = script_command(&app, &package_manager, &script_name)?;
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

    if path.join("composer.json").exists() {
        managers.push("composer".to_string());
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
            kill_script,
            kill_project_scripts,
            get_running_scripts,
            list_projects,
            detect_package_managers,
            load_pins,
            save_pins,
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
