#![allow(unused_imports)]
//! Java support: plain Java, Maven, Gradle and Spring Boot scaffolds and project reading.

use std::path::{Path, PathBuf};

use crate::common::*;

pub(crate) fn java_bin() -> String {
    if cfg!(target_os = "windows") {
        "java.exe".to_string()
    } else {
        "java".to_string()
    }
}

pub(crate) fn javac_bin() -> String {
    if cfg!(target_os = "windows") {
        "javac.exe".to_string()
    } else {
        "javac".to_string()
    }
}

pub(crate) fn maven_bin() -> String {
    if cfg!(target_os = "windows") {
        "mvn.cmd".to_string()
    } else {
        "mvn".to_string()
    }
}

pub(crate) fn gradle_bin() -> String {
    if cfg!(target_os = "windows") {
        "gradle.bat".to_string()
    } else {
        "gradle".to_string()
    }
}

pub(crate) const SPRING_BOOT_VERSION: &str = "3.5.14";

pub(crate) fn java_basic_steps(
    project_name: &str,
    parent_dir: &Path,
    target_dir: &Path,
) -> Vec<CreationStep> {
    vec![write_files_step(
        "Create Java starter files",
        vec![
            (
                target_dir.join("Main.java"),
                r#"public class Main {
    public static void main(String[] args) {
        System.out.println("Hello from Java");
    }
}
"#
                .to_string(),
            ),
            (
                target_dir.join("README.md"),
                format!("# {}\n\nCreated with ProLaunch.\n", project_name),
            ),
        ],
        parent_dir,
        format!("$ ProLaunch scaffold {}", project_name),
    )]
}

pub(crate) fn maven_java_main_source() -> &'static str {
    r#"package com.prolaunch.app;

public class App {
    public static void main(String[] args) {
        System.out.println("Hello from Maven Java");
    }
}
"#
}

pub(crate) fn maven_pom(project_name: &str) -> String {
    format!(
        r#"<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 https://maven.apache.org/xsd/maven-4.0.0.xsd">
  <modelVersion>4.0.0</modelVersion>

  <groupId>com.prolaunch</groupId>
  <artifactId>{}</artifactId>
  <version>0.1.0</version>

  <properties>
    <maven.compiler.source>17</maven.compiler.source>
    <maven.compiler.target>17</maven.compiler.target>
    <project.build.sourceEncoding>UTF-8</project.build.sourceEncoding>
  </properties>

  <build>
    <plugins>
      <plugin>
        <groupId>org.codehaus.mojo</groupId>
        <artifactId>exec-maven-plugin</artifactId>
        <version>3.5.0</version>
        <configuration>
          <mainClass>com.prolaunch.app.App</mainClass>
        </configuration>
      </plugin>
    </plugins>
  </build>
</project>
"#,
        project_name
    )
}

pub(crate) fn maven_java_steps(
    project_name: &str,
    parent_dir: &Path,
    target_dir: &Path,
) -> Vec<CreationStep> {
    vec![write_files_step(
        "Create Maven starter files",
        vec![
            (target_dir.join("pom.xml"), maven_pom(project_name)),
            (
                target_dir.join("src/main/java/com/prolaunch/app/App.java"),
                maven_java_main_source().to_string(),
            ),
            (
                target_dir.join("README.md"),
                format!("# {}\n\nCreated with ProLaunch.\n", project_name),
            ),
        ],
        parent_dir,
        format!("$ ProLaunch scaffold {}", project_name),
    )]
}

pub(crate) fn gradle_build_file() -> &'static str {
    r#"plugins {
    id 'application'
}

repositories {
    mavenCentral()
}

java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
}

application {
    mainClass = 'com.prolaunch.app.App'
}
"#
}

pub(crate) fn gradle_settings(project_name: &str) -> String {
    format!("rootProject.name = '{}'\n", project_name)
}

pub(crate) fn gradle_java_steps(
    project_name: &str,
    parent_dir: &Path,
    target_dir: &Path,
) -> Vec<CreationStep> {
    vec![write_files_step(
        "Create Gradle starter files",
        vec![
            (
                target_dir.join("settings.gradle"),
                gradle_settings(project_name),
            ),
            (
                target_dir.join("build.gradle"),
                gradle_build_file().to_string(),
            ),
            (
                target_dir.join("src/main/java/com/prolaunch/app/App.java"),
                maven_java_main_source().to_string(),
            ),
            (
                target_dir.join("README.md"),
                format!("# {}\n\nCreated with ProLaunch.\n", project_name),
            ),
        ],
        parent_dir,
        format!("$ ProLaunch scaffold {}", project_name),
    )]
}

pub(crate) fn spring_boot_main_source() -> &'static str {
    r#"package com.prolaunch.app;

import java.util.Map;

import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RestController;

@SpringBootApplication
@RestController
public class App {
    public static void main(String[] args) {
        SpringApplication.run(App.class, args);
    }

    @GetMapping("/")
    public Map<String, String> home() {
        return Map.of("message", "Hello from Spring Boot");
    }
}
"#
}

pub(crate) fn spring_boot_pom(project_name: &str) -> String {
    format!(
        r#"<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 https://maven.apache.org/xsd/maven-4.0.0.xsd">
  <modelVersion>4.0.0</modelVersion>

  <parent>
    <groupId>org.springframework.boot</groupId>
    <artifactId>spring-boot-starter-parent</artifactId>
    <version>{}</version>
    <relativePath/>
  </parent>

  <groupId>com.prolaunch</groupId>
  <artifactId>{}</artifactId>
  <version>0.1.0</version>

  <properties>
    <java.version>17</java.version>
  </properties>

  <dependencies>
    <dependency>
      <groupId>org.springframework.boot</groupId>
      <artifactId>spring-boot-starter-web</artifactId>
    </dependency>
  </dependencies>

  <build>
    <plugins>
      <plugin>
        <groupId>org.springframework.boot</groupId>
        <artifactId>spring-boot-maven-plugin</artifactId>
      </plugin>
    </plugins>
  </build>
</project>
"#,
        SPRING_BOOT_VERSION, project_name
    )
}

pub(crate) fn spring_boot_steps(
    project_name: &str,
    parent_dir: &Path,
    target_dir: &Path,
) -> Vec<CreationStep> {
    vec![
        write_files_step(
            "Create Spring Boot starter files",
            vec![
                (target_dir.join("pom.xml"), spring_boot_pom(project_name)),
                (
                    target_dir.join("src/main/java/com/prolaunch/app/App.java"),
                    spring_boot_main_source().to_string(),
                ),
                (
                    target_dir.join("README.md"),
                    format!("# {}\n\nCreated with ProLaunch.\n", project_name),
                ),
            ],
            parent_dir,
            format!("$ ProLaunch scaffold {}", project_name),
        ),
        creation_step(
            "Install Spring Boot dependencies",
            maven_bin(),
            vec!["-q", "dependency:resolve"],
            target_dir,
        ),
    ]
}

pub(crate) fn find_java_main_file(project_dir: &Path) -> Option<PathBuf> {
    let main_path = project_dir.join("Main.java");
    if main_path.exists() {
        return Some(main_path);
    }

    let entries = std::fs::read_dir(project_dir).ok()?;
    entries.filter_map(Result::ok).find_map(|entry| {
        let path = entry.path();
        let is_java_file = path.extension().and_then(|ext| ext.to_str()) == Some("java");
        if !is_java_file {
            return None;
        }

        std::fs::read_to_string(&path)
            .ok()
            .filter(|content| content.contains("static void main"))
            .map(|_| path)
    })
}

pub(crate) fn java_main_class(main_file: &Path) -> String {
    main_file
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Main")
        .to_string()
}

pub(crate) fn java_main_command(main_file: &Path) -> String {
    let file_name = main_file
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Main.java")
        .to_string();
    let class_name = java_main_class(main_file);
    format!("javac {} && java {}", file_name, class_name)
}

pub(crate) fn java_main_command_line(main_file: &Path) -> String {
    let file_name = main_file
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Main.java")
        .to_string();
    let class_name = java_main_class(main_file);
    format!(
        "{} {} && {} {}",
        shell_quote(&javac_bin()),
        shell_quote(&file_name),
        shell_quote(&java_bin()),
        shell_quote(&class_name)
    )
}

pub(crate) fn java_main_scripts(main_file: &Path) -> Vec<ScriptInfo> {
    let source = main_file
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Main.java")
        .to_string();
    let command = java_main_command(main_file);

    vec![ScriptInfo {
        name: "start".to_string(),
        command,
        package_manager: Some("java".to_string()),
        source: Some(source),
    }]
}

pub(crate) fn xml_first_text(content: &str, tag: &str) -> Option<String> {
    let start = format!("<{}>", tag);
    let end = format!("</{}>", tag);
    content.find(&start).and_then(|start_index| {
        let value_start = start_index + start.len();
        content[value_start..].find(&end).map(|end_index| {
            content[value_start..value_start + end_index]
                .trim()
                .to_string()
        })
    })
}

pub(crate) fn maven_project_name(project_dir: &Path, content: &str) -> String {
    let project_section = content
        .find("</parent>")
        .map(|index| &content[index + "</parent>".len()..])
        .unwrap_or(content);

    xml_first_text(project_section, "artifactId")
        .unwrap_or_else(|| python_folder_project_name(project_dir))
}

pub(crate) fn maven_command(project_dir: &Path) -> ToolCommand {
    let wrapper = if cfg!(target_os = "windows") {
        project_dir.join("mvnw.cmd")
    } else {
        project_dir.join("mvnw")
    };

    if wrapper.exists() {
        ToolCommand {
            program: wrapper.to_string_lossy().to_string(),
            args: Vec::new(),
        }
    } else {
        ToolCommand {
            program: maven_bin(),
            args: Vec::new(),
        }
    }
}

pub(crate) fn gradle_command(project_dir: &Path) -> ToolCommand {
    let wrapper = if cfg!(target_os = "windows") {
        project_dir.join("gradlew.bat")
    } else {
        project_dir.join("gradlew")
    };

    if wrapper.exists() {
        ToolCommand {
            program: wrapper.to_string_lossy().to_string(),
            args: Vec::new(),
        }
    } else {
        ToolCommand {
            program: gradle_bin(),
            args: Vec::new(),
        }
    }
}

pub(crate) fn maven_script_args(content: &str, script_name: &str) -> Option<Vec<&'static str>> {
    let is_spring = content.contains("spring-boot-maven-plugin")
        || content.contains("spring-boot-starter")
        || content.contains("spring-boot");
    let has_exec = content.contains("exec-maven-plugin");

    match script_name {
        "dev" | "start" if is_spring => Some(vec!["spring-boot:run"]),
        "dev" | "start" if has_exec => Some(vec!["exec:java"]),
        "test" => Some(vec!["test"]),
        "package" => Some(vec!["package"]),
        "clean" => Some(vec!["clean"]),
        _ => None,
    }
}

pub(crate) fn maven_scripts(project_dir: &Path, content: &str) -> Vec<ScriptInfo> {
    let tool = maven_command(project_dir);
    let display = command_display(&tool);
    let names = ["dev", "start", "test", "package", "clean"];

    names
        .into_iter()
        .filter_map(|name| {
            maven_script_args(content, name).map(|args| ScriptInfo {
                name: name.to_string(),
                command: format!("{} {}", display, args.join(" ")),
                package_manager: Some("maven".to_string()),
                source: Some("pom.xml".to_string()),
            })
        })
        .collect()
}

pub(crate) fn read_maven_project(
    project_path: String,
    project_dir: &Path,
    pom_path: &Path,
) -> Result<ProjectInfo, String> {
    let content =
        std::fs::read_to_string(pom_path).map_err(|e| format!("Failed to read pom.xml: {}", e))?;
    let name = maven_project_name(project_dir, &content);

    Ok(ProjectInfo {
        name,
        path: project_path,
        scripts: maven_scripts(project_dir, &content),
        package_manager: "maven".to_string(),
    })
}

pub(crate) fn gradle_project_name(project_dir: &Path) -> String {
    let settings_path = if project_dir.join("settings.gradle").exists() {
        project_dir.join("settings.gradle")
    } else {
        project_dir.join("settings.gradle.kts")
    };

    std::fs::read_to_string(settings_path)
        .ok()
        .and_then(|content| {
            content.lines().find_map(|line| {
                let trimmed = line.trim();
                trimmed
                    .strip_prefix("rootProject.name")
                    .and_then(|rest| rest.split_once('='))
                    .map(|(_, value)| value.trim().trim_matches(['"', '\'']).to_string())
            })
        })
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| python_folder_project_name(project_dir))
}

pub(crate) fn gradle_script_args(content: &str, script_name: &str) -> Option<Vec<&'static str>> {
    let is_spring = content.contains("org.springframework.boot")
        || content.contains("spring-boot-starter")
        || content.contains("bootRun");
    let has_application = content.contains("application") || content.contains("mainClass");

    match script_name {
        "dev" | "start" if is_spring => Some(vec!["bootRun"]),
        "dev" | "start" if has_application => Some(vec!["run"]),
        "test" => Some(vec!["test"]),
        "build" => Some(vec!["build"]),
        "clean" => Some(vec!["clean"]),
        _ => None,
    }
}

pub(crate) fn gradle_scripts(project_dir: &Path, content: &str, source: &str) -> Vec<ScriptInfo> {
    let tool = gradle_command(project_dir);
    let display = command_display(&tool);
    let names = ["dev", "start", "test", "build", "clean"];

    names
        .into_iter()
        .filter_map(|name| {
            gradle_script_args(content, name).map(|args| ScriptInfo {
                name: name.to_string(),
                command: format!("{} {}", display, args.join(" ")),
                package_manager: Some("gradle".to_string()),
                source: Some(source.to_string()),
            })
        })
        .collect()
}

pub(crate) fn read_gradle_project(
    project_path: String,
    project_dir: &Path,
    build_path: &Path,
) -> Result<ProjectInfo, String> {
    let content = std::fs::read_to_string(build_path)
        .map_err(|e| format!("Failed to read Gradle build file: {}", e))?;
    let source = build_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("build.gradle");

    Ok(ProjectInfo {
        name: gradle_project_name(project_dir),
        path: project_path,
        scripts: gradle_scripts(project_dir, &content, source),
        package_manager: "gradle".to_string(),
    })
}

pub(crate) fn java_script_command_line(
    project_dir: &Path,
    script_name: &str,
) -> Result<String, String> {
    if script_name != "start" {
        return Err(format!("Java script '{}' was not found", script_name));
    }

    let main_file = find_java_main_file(project_dir)
        .ok_or_else(|| "Java main file was not found".to_string())?;
    Ok(java_main_command_line(&main_file))
}

pub(crate) fn maven_script_command_line(
    project_dir: &Path,
    script_name: &str,
) -> Result<String, String> {
    let pom_path = project_dir.join("pom.xml");
    let content =
        std::fs::read_to_string(&pom_path).map_err(|e| format!("Failed to read pom.xml: {}", e))?;
    let args = maven_script_args(&content, script_name)
        .ok_or_else(|| format!("Maven script '{}' was not found", script_name))?;
    let tool = maven_command(project_dir);
    Ok(tool_command_line(
        &tool,
        &args.into_iter().map(str::to_string).collect::<Vec<_>>(),
    ))
}

pub(crate) fn gradle_script_command_line(
    project_dir: &Path,
    script_name: &str,
) -> Result<String, String> {
    let build_path = if project_dir.join("build.gradle").exists() {
        project_dir.join("build.gradle")
    } else {
        project_dir.join("build.gradle.kts")
    };
    let content = std::fs::read_to_string(&build_path)
        .map_err(|e| format!("Failed to read Gradle build file: {}", e))?;
    let args = gradle_script_args(&content, script_name)
        .ok_or_else(|| format!("Gradle script '{}' was not found", script_name))?;
    let tool = gradle_command(project_dir);
    Ok(tool_command_line(
        &tool,
        &args.into_iter().map(str::to_string).collect::<Vec<_>>(),
    ))
}
