#![allow(unused_imports)]
//! PHP / Composer support: Composer bootstrapping, scaffolds, project reading.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::common::*;

pub(crate) fn composer_bin() -> String {
    package_manager_command("composer")
}

pub(crate) fn php_bin() -> String {
    "php".to_string()
}

pub(crate) fn app_managed_composer_path(app: &AppHandle) -> Result<PathBuf, String> {
    let mut dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    dir.push("tools");
    dir.push("composer");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to prepare tool cache: {}", e))?;
    Ok(dir.join("composer.phar"))
}

pub(crate) fn managed_composer_command(composer_path: &Path) -> ToolCommand {
    ToolCommand {
        program: php_bin(),
        args: vec![composer_path.to_string_lossy().to_string()],
    }
}

pub(crate) fn verify_managed_composer(
    app: &AppHandle,
    creation_id: &str,
    composer_path: &Path,
) -> bool {
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

pub(crate) fn composer_install_steps(composer_path: &Path) -> Result<Vec<CreationStep>, String> {
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

pub(crate) fn prepare_composer(
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

pub(crate) fn composer_create_project_steps(
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

pub(crate) fn composer_script_command(cmd: &serde_json::Value) -> String {
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

pub(crate) fn is_composer_lifecycle_script(name: &str) -> bool {
    name == "auto-scripts" || name.starts_with("pre-") || name.starts_with("post-")
}

pub(crate) fn composer_project_name(project_dir: &Path, parsed: &serde_json::Value) -> String {
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

pub(crate) fn read_composer_project(
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
                    package_manager: Some("composer".to_string()),
                    source: Some("composer.json".to_string()),
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
