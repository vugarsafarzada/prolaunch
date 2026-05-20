#![allow(unused_imports)]
//! Ruby support: plain Ruby, Sinatra and Rails scaffolds and Gemfile project reading.

use std::path::Path;

use crate::common::*;

pub(crate) fn ruby_bin() -> String {
    if cfg!(target_os = "windows") {
        "ruby.exe".to_string()
    } else {
        "ruby".to_string()
    }
}

pub(crate) fn bundle_bin() -> String {
    if cfg!(target_os = "windows") {
        "bundle.bat".to_string()
    } else {
        "bundle".to_string()
    }
}

pub(crate) fn ruby_basic_steps(
    project_name: &str,
    parent_dir: &Path,
    target_dir: &Path,
) -> Vec<CreationStep> {
    vec![write_files_step(
        "Create Ruby starter files",
        vec![
            (
                target_dir.join("main.rb"),
                r#"puts "Hello from Ruby"
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

pub(crate) fn sinatra_gemfile() -> &'static str {
    r#"source "https://rubygems.org"

gem "puma"
gem "sinatra"
"#
}

pub(crate) fn sinatra_app_source() -> &'static str {
    r#"require "json"
require "sinatra"

set :bind, "0.0.0.0"
set :port, ENV.fetch("PORT", 4567)

get "/" do
  content_type :json
  { message: "Hello from Sinatra" }.to_json
end
"#
}

pub(crate) fn sinatra_steps(
    project_name: &str,
    parent_dir: &Path,
    target_dir: &Path,
) -> Vec<CreationStep> {
    vec![
        write_files_step(
            "Create Sinatra starter files",
            vec![
                (target_dir.join("Gemfile"), sinatra_gemfile().to_string()),
                (target_dir.join("app.rb"), sinatra_app_source().to_string()),
                (
                    target_dir.join("config.ru"),
                    "require_relative \"app\"\nrun Sinatra::Application\n".to_string(),
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
            "Install Ruby dependencies",
            bundle_bin(),
            vec!["install", "--path", "vendor/bundle"],
            target_dir,
        ),
    ]
}

pub(crate) fn rails_gemfile() -> &'static str {
    r#"source "https://rubygems.org"

gem "rails", "~> 6.1.7"
gem "puma", "~> 5.6"
"#
}

pub(crate) fn rails_boot_source() -> &'static str {
    r#"ENV["BUNDLE_GEMFILE"] ||= File.expand_path("../Gemfile", __dir__)

require "bundler/setup"
"#
}

pub(crate) fn rails_application_source() -> &'static str {
    r#"require_relative "boot"

require "logger"
require "rails"
require "active_model/railtie"
require "action_controller/railtie"
require "rails/test_unit/railtie"

Bundler.require(*Rails.groups)

module ProlaunchApp
  class Application < Rails::Application
    config.load_defaults 6.1
    config.api_only = true
    config.require_master_key = false
  end
end
"#
}

pub(crate) fn rails_environment_source() -> &'static str {
    r#"require_relative "application"

Rails.application.initialize!
"#
}

pub(crate) fn rails_development_source() -> &'static str {
    r#"Rails.application.configure do
  config.cache_classes = false
  config.eager_load = false
  config.consider_all_requests_local = true
end
"#
}

pub(crate) fn rails_test_source() -> &'static str {
    r#"Rails.application.configure do
  config.cache_classes = true
  config.eager_load = false
  config.public_file_server.enabled = true
  config.consider_all_requests_local = true
end
"#
}

pub(crate) fn rails_routes_source() -> &'static str {
    r#"Rails.application.routes.draw do
  root "home#index"
end
"#
}

pub(crate) fn rails_config_ru_source() -> &'static str {
    r#"require_relative "config/environment"

run Rails.application
Rails.application.load_server
"#
}

pub(crate) fn rails_puma_source() -> &'static str {
    r#"port ENV.fetch("PORT", 3000)
environment ENV.fetch("RAILS_ENV", "development")
"#
}

pub(crate) fn rails_application_controller_source() -> &'static str {
    r#"class ApplicationController < ActionController::API
end
"#
}

pub(crate) fn rails_home_controller_source() -> &'static str {
    r#"class HomeController < ApplicationController
  def index
    render json: { message: "Hello from Rails" }
  end
end
"#
}

pub(crate) fn rails_bin_source() -> &'static str {
    r#"#!/usr/bin/env ruby

APP_PATH = File.expand_path("../config/application", __dir__)
require_relative "../config/boot"
require "rails/commands"
"#
}

pub(crate) fn rails_steps(
    project_name: &str,
    parent_dir: &Path,
    target_dir: &Path,
) -> Vec<CreationStep> {
    vec![
        write_files_step(
            "Create Rails starter files",
            vec![
                (target_dir.join("Gemfile"), rails_gemfile().to_string()),
                (
                    target_dir.join("config").join("boot.rb"),
                    rails_boot_source().to_string(),
                ),
                (
                    target_dir.join("config").join("application.rb"),
                    rails_application_source().to_string(),
                ),
                (
                    target_dir.join("config").join("environment.rb"),
                    rails_environment_source().to_string(),
                ),
                (
                    target_dir
                        .join("config")
                        .join("environments")
                        .join("development.rb"),
                    rails_development_source().to_string(),
                ),
                (
                    target_dir
                        .join("config")
                        .join("environments")
                        .join("test.rb"),
                    rails_test_source().to_string(),
                ),
                (
                    target_dir.join("config").join("routes.rb"),
                    rails_routes_source().to_string(),
                ),
                (
                    target_dir.join("config").join("puma.rb"),
                    rails_puma_source().to_string(),
                ),
                (
                    target_dir.join("config.ru"),
                    rails_config_ru_source().to_string(),
                ),
                (
                    target_dir
                        .join("app")
                        .join("controllers")
                        .join("application_controller.rb"),
                    rails_application_controller_source().to_string(),
                ),
                (
                    target_dir
                        .join("app")
                        .join("controllers")
                        .join("home_controller.rb"),
                    rails_home_controller_source().to_string(),
                ),
                (
                    target_dir.join("bin").join("rails"),
                    rails_bin_source().to_string(),
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
            "Install Rails dependencies",
            bundle_bin(),
            vec!["install", "--path", "vendor/bundle"],
            target_dir,
        ),
    ]
}

pub(crate) fn ruby_main_script() -> ScriptInfo {
    ScriptInfo {
        name: "start".to_string(),
        command: "ruby main.rb".to_string(),
        package_manager: Some("ruby".to_string()),
        source: Some("main.rb".to_string()),
    }
}

pub(crate) fn is_rails_project(project_dir: &Path, gemfile: &str) -> bool {
    project_dir.join("config").join("application.rb").exists()
        || project_dir.join("bin").join("rails").exists()
        || gemfile.contains("rails")
}

pub(crate) fn is_sinatra_project(project_dir: &Path, gemfile: &str) -> bool {
    project_dir.join("app.rb").exists() || gemfile.contains("sinatra")
}

pub(crate) fn rails_uses_active_record(project_dir: &Path, gemfile: &str) -> bool {
    let application_path = project_dir.join("config").join("application.rb");
    let application = std::fs::read_to_string(application_path).unwrap_or_default();

    application.contains("rails/all")
        || application.contains("active_record/railtie")
        || gemfile.contains("sqlite3")
        || gemfile.contains("pg")
        || gemfile.contains("mysql2")
}

pub(crate) fn ruby_scripts(project_dir: &Path, gemfile: Option<&str>) -> Vec<ScriptInfo> {
    let mut scripts = Vec::new();

    if let Some(content) = gemfile {
        if is_rails_project(project_dir, content) {
            scripts.extend([
                (
                    "dev",
                    "bundle exec ruby -rlogger bin/rails server",
                    "Gemfile",
                ),
                (
                    "start",
                    "bundle exec ruby -rlogger bin/rails server",
                    "Gemfile",
                ),
                (
                    "console",
                    "bundle exec ruby -rlogger bin/rails console",
                    "Gemfile",
                ),
                (
                    "test",
                    "bundle exec ruby -rlogger bin/rails test",
                    "Gemfile",
                ),
                (
                    "routes",
                    "bundle exec ruby -rlogger bin/rails routes",
                    "Gemfile",
                ),
            ]);
            if rails_uses_active_record(project_dir, content) {
                scripts.push((
                    "db-migrate",
                    "bundle exec ruby -rlogger bin/rails db:migrate",
                    "Gemfile",
                ));
            }
        } else if is_sinatra_project(project_dir, content) {
            scripts.extend([
                ("dev", "bundle exec ruby app.rb", "Gemfile"),
                ("start", "bundle exec ruby app.rb", "Gemfile"),
            ]);
        } else if project_dir.join("main.rb").exists() {
            scripts.push(("start", "ruby main.rb", "main.rb"));
        }

        scripts.push((
            "bundle-install",
            "bundle install --path vendor/bundle",
            "Gemfile",
        ));
    } else if project_dir.join("main.rb").exists() {
        scripts.push(("start", "ruby main.rb", "main.rb"));
    }

    scripts
        .into_iter()
        .map(|(name, command, source)| ScriptInfo {
            name: name.to_string(),
            command: command.to_string(),
            package_manager: Some("ruby".to_string()),
            source: Some(source.to_string()),
        })
        .collect()
}

pub(crate) fn read_ruby_project(
    project_path: String,
    project_dir: &Path,
    gemfile_path: &Path,
) -> Result<ProjectInfo, String> {
    let content = std::fs::read_to_string(gemfile_path)
        .map_err(|e| format!("Failed to read Gemfile: {}", e))?;

    Ok(ProjectInfo {
        name: python_folder_project_name(project_dir),
        path: project_path,
        scripts: ruby_scripts(project_dir, Some(&content)),
        package_manager: "ruby".to_string(),
    })
}

pub(crate) fn ruby_script_command_line(
    project_dir: &Path,
    script_name: &str,
) -> Result<String, String> {
    let gemfile_path = project_dir.join("Gemfile");
    let gemfile = if gemfile_path.exists() {
        Some(
            std::fs::read_to_string(&gemfile_path)
                .map_err(|e| format!("Failed to read Gemfile: {}", e))?,
        )
    } else {
        None
    };
    let scripts = ruby_scripts(project_dir, gemfile.as_deref());

    scripts
        .into_iter()
        .find(|script| script.name == script_name)
        .map(|script| script.command)
        .ok_or_else(|| format!("Ruby script '{}' was not found", script_name))
}
