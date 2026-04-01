use anyhow::Result;
use colored::*;
use serde::Serialize;

use crate::config::{load_config, ConfigPaths, validate_module};
use crate::package::PackageManager;

#[derive(Serialize)]
struct ValidationOutput {
    valid: bool,
    errors: usize,
    warnings: usize,
    modules: Vec<ModuleValidation>,
}

#[derive(Serialize)]
struct ModuleValidation {
    name: String,
    errors: Vec<String>,
    warnings: Vec<String>,
}

pub fn run(paths: &ConfigPaths, check_packages: bool, json: bool) -> Result<()> {
    let config = match load_config(paths) {
        Ok(c) => c,
        Err(e) => {
            if json {
                println!("{{\"valid\":false,\"error\":\"{}\" }}", e);
            } else {
                println!("{} Failed to load config: {}", "✗".red(), e);
            }
            return Ok(());
        }
    };

    let pkg_manager = PackageManager::new(paths.clone());
    let mut total_errors = 0;
    let mut total_warnings = 0;
    let mut module_validations = Vec::new();

    for module_name in &config.enabled_modules {
        let module_path = match pkg_manager.find_module_path(module_name) {
            Some(p) => p,
            None => {
                let mv = ModuleValidation {
                    name: module_name.clone(),
                    errors: vec![format!("Module '{}' not found", module_name)],
                    warnings: Vec::new(),
                };
                total_errors += 1;
                module_validations.push(mv);
                continue;
            }
        };

        match crate::config::load_module(&module_path) {
            Ok(module) => {
                let result = validate_module(&module, module_name);

                if check_packages {
                    // Check each package exists in xbps repos
                    for pkg in result.errors.iter() {
                        // already collected above
                        let _ = pkg;
                    }
                }

                if !json && !result.is_clean() {
                    println!("  {} Module '{}':", "→".blue(), module_name);
                    for e in &result.errors {
                        println!("    {} {}", "✗".red(), e);
                    }
                    for w in &result.warnings {
                        println!("    {} {}", "⚠".yellow(), w);
                    }
                }

                total_errors += result.errors.len();
                total_warnings += result.warnings.len();

                module_validations.push(ModuleValidation {
                    name: module_name.clone(),
                    errors: result.errors,
                    warnings: result.warnings,
                });
            }
            Err(e) => {
                let mv = ModuleValidation {
                    name: module_name.clone(),
                    errors: vec![format!("Failed to load: {}", e)],
                    warnings: Vec::new(),
                };
                total_errors += 1;
                module_validations.push(mv);
            }
        }
    }

    if check_packages && !json {
        println!("{}", "Checking package availability in xbps repos...".blue());
        let declared = pkg_manager.get_declared_packages(&config)?;
        for pkg in &declared {
            if pkg.package_type == crate::config::PackageType::Xbps {
                let exists = pkg_manager.xbps_pkg_exists(&pkg.name);
                if !exists {
                    println!("  {} Package not found in repos: {}", "!".yellow(), pkg.name);
                    total_warnings += 1;
                }
            }
        }
    }

    if json {
        let output = ValidationOutput {
            valid: total_errors == 0,
            errors: total_errors,
            warnings: total_warnings,
            modules: module_validations,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!();
    if total_errors == 0 {
        println!(
            "{} Validation passed ({} warning(s))",
            "✓".green(),
            total_warnings
        );
    } else {
        println!(
            "{} Validation failed: {} error(s), {} warning(s)",
            "✗".red(),
            total_errors,
            total_warnings
        );
    }

    Ok(())
}
