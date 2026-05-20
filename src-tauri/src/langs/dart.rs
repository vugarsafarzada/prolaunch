#![allow(unused_imports)]
//! Dart / Flutter support: SDK detection, scaffolds, pubspec project reading.

use std::path::Path;

use crate::common::*;

pub(crate) fn dart_bin() -> String {
    if cfg!(target_os = "windows") {
        "dart.exe".to_string()
    } else {
        "dart".to_string()
    }
}

pub(crate) fn flutter_bin() -> String {
    if cfg!(target_os = "windows") {
        "flutter.bat".to_string()
    } else {
        "flutter".to_string()
    }
}

pub(crate) fn is_flutter_pubspec(content: &str) -> bool {
    content.contains("sdk: flutter") || content.lines().any(|line| line.trim() == "flutter:")
}

pub(crate) fn preferred_pub_package_manager(path: &Path) -> String {
    match std::fs::read_to_string(path.join("pubspec.yaml")) {
        Ok(content) if is_flutter_pubspec(&content) => "flutter".to_string(),
        _ => "dart".to_string(),
    }
}

pub(crate) fn dart_console_steps(project_name: &str, parent_dir: &Path) -> Vec<CreationStep> {
    vec![creation_step(
        "Create Dart console project",
        dart_bin(),
        vec!["create", "-t", "console-simple", project_name],
        parent_dir,
    )]
}

pub(crate) fn flutter_app_steps(project_name: &str, parent_dir: &Path) -> Vec<CreationStep> {
    vec![creation_step(
        "Create Flutter project",
        flutter_bin(),
        vec!["create", project_name],
        parent_dir,
    )]
}

pub(crate) fn yaml_top_level_string(content: &str, key: &str) -> Option<String> {
    let prefix = format!("{}:", key);
    content.lines().find_map(|line| {
        if line.starts_with(char::is_whitespace) {
            return None;
        }

        let trimmed = line.trim();
        trimmed
            .strip_prefix(&prefix)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.trim_matches(['"', '\'']).to_string())
    })
}

pub(crate) fn pub_project_name(project_dir: &Path, content: &str) -> String {
    yaml_top_level_string(content, "name")
        .unwrap_or_else(|| python_folder_project_name(project_dir))
}

pub(crate) fn pub_scripts(is_flutter: bool, has_tests: bool) -> Vec<ScriptInfo> {
    let mut scripts = if is_flutter {
        vec![
            ("dev", "flutter run"),
            ("start", "flutter run"),
            ("pub-get", "flutter pub get"),
            ("build-web", "flutter build web"),
        ]
    } else {
        vec![("start", "dart run"), ("pub-get", "dart pub get")]
    };

    if is_flutter || has_tests {
        scripts.push((
            "test",
            if is_flutter {
                "flutter test"
            } else {
                "dart test"
            },
        ));
    }

    let package_manager = if is_flutter { "flutter" } else { "dart" };
    scripts
        .into_iter()
        .map(|(name, command)| ScriptInfo {
            name: name.to_string(),
            command: command.to_string(),
            package_manager: Some(package_manager.to_string()),
            source: Some("pubspec.yaml".to_string()),
        })
        .collect()
}

pub(crate) fn read_pub_project(
    project_path: String,
    project_dir: &Path,
    pubspec_path: &Path,
) -> Result<ProjectInfo, String> {
    let content = std::fs::read_to_string(pubspec_path)
        .map_err(|e| format!("Failed to read pubspec.yaml: {}", e))?;
    let is_flutter = is_flutter_pubspec(&content);
    let package_manager = if is_flutter { "flutter" } else { "dart" }.to_string();
    let has_tests = project_dir.join("test").exists() || content.contains("\n  test:");

    Ok(ProjectInfo {
        name: pub_project_name(project_dir, &content),
        path: project_path,
        scripts: pub_scripts(is_flutter, has_tests),
        package_manager,
    })
}

pub(crate) fn pub_script_command_line(
    project_dir: &Path,
    package_manager: &str,
    script_name: &str,
) -> Result<String, String> {
    let pubspec_path = project_dir.join("pubspec.yaml");
    let content = std::fs::read_to_string(&pubspec_path)
        .map_err(|e| format!("Failed to read pubspec.yaml: {}", e))?;
    let is_flutter = package_manager == "flutter" || is_flutter_pubspec(&content);
    let has_tests = project_dir.join("test").exists() || content.contains("\n  test:");

    pub_scripts(is_flutter, has_tests)
        .into_iter()
        .find(|script| script.name == script_name)
        .map(|script| script.command)
        .ok_or_else(|| {
            format!(
                "{} script '{}' was not found in pubspec.yaml defaults",
                if is_flutter { "Flutter" } else { "Dart" },
                script_name
            )
        })
}
