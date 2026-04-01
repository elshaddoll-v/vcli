use anyhow::Result;
use colored::*;
use serde::Serialize;

use crate::config::{load_config, ConfigPaths};
use crate::package::PackageManager;

#[derive(Serialize)]
struct FindResult {
    package: String,
    found: bool,
    location: Option<String>,
    module: Option<String>,
    file: Option<String>,
}

pub fn run(paths: &ConfigPaths, package: &str, json: bool) -> Result<()> {
    let config = load_config(paths)?;
    let pkg_manager = PackageManager::new(paths.clone());

    let mut results: Vec<FindResult> = Vec::new();

    // Check base.yaml
    let base_file = paths.base_packages_file();
    if base_file.exists() {
        if let Ok(list) = crate::config::load_package_list(&base_file) {
            if list.packages.iter().any(|p| p.name() == package) {
                results.push(FindResult {
                    package: package.to_string(),
                    found: true,
                    location: Some("base module".to_string()),
                    module: Some("base".to_string()),
                    file: Some(base_file.display().to_string()),
                });
            }
        }
    }

    // Check declared-packages.yaml
    let decl_file = paths.modules_dir().join("declared-packages.yaml");
    if decl_file.exists() {
        if let Ok(list) = crate::config::load_package_list(&decl_file) {
            if list.packages.iter().any(|p| p.name() == package) {
                results.push(FindResult {
                    package: package.to_string(),
                    found: true,
                    location: Some("declared-packages (vcli install)".to_string()),
                    module: Some("declared-packages".to_string()),
                    file: Some(decl_file.display().to_string()),
                });
            }
        }
    }

    // Check system-packages
    let sys_pkg_file = paths
        .modules_dir()
        .join(format!("system-packages-{}/packages.yaml", config.host));
    if sys_pkg_file.exists() {
        if let Ok(list) = crate::config::load_package_list(&sys_pkg_file) {
            if list.packages.iter().any(|p| p.name() == package) {
                results.push(FindResult {
                    package: package.to_string(),
                    found: true,
                    location: Some("system-packages (vcli merge)".to_string()),
                    module: Some("system-packages".to_string()),
                    file: Some(sys_pkg_file.display().to_string()),
                });
            }
        }
    }

    // Check host config
    let config_path = crate::config::resolve_config_path(paths)?;
    if config.packages.iter().any(|p| p.name() == package) {
        results.push(FindResult {
            package: package.to_string(),
            found: true,
            location: Some("host config".to_string()),
            module: None,
            file: Some(config_path.display().to_string()),
        });
    }

    // Check enabled modules
    for module_name in &config.enabled_modules {
        if let Some(module_path) = pkg_manager.find_module_path(module_name) {
            if let Ok(module) = crate::config::load_module(&module_path) {
                if module.packages().iter().any(|p| p.name() == package) {
                    results.push(FindResult {
                        package: package.to_string(),
                        found: true,
                        location: Some(format!("module: {}", module_name)),
                        module: Some(module_name.clone()),
                        file: Some(module_path.display().to_string()),
                    });
                }
            }
        }
    }

    if json {
        if results.is_empty() {
            let not_found = FindResult {
                package: package.to_string(),
                found: false,
                location: None,
                module: None,
                file: None,
            };
            println!("{}", serde_json::to_string_pretty(&not_found)?);
        } else {
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
        return Ok(());
    }

    if results.is_empty() {
        println!(
            "{} '{}' is not declared in any config file.",
            "✗".red(),
            package
        );
    } else {
        println!(
            "{} '{}' found in {} location(s):",
            "✓".green(),
            package,
            results.len()
        );
        for r in &results {
            println!();
            println!("  {} {}", "→".blue(), r.location.as_deref().unwrap_or(""));
            println!("    File: {}", r.file.as_deref().unwrap_or(""));
        }
    }

    Ok(())
}
