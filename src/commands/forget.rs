use anyhow::Result;
use colored::*;

use crate::config::{load_config, ConfigPaths};
use crate::package::PackageManager;

pub fn run(paths: &ConfigPaths, package: &str) -> Result<()> {
    let config = load_config(paths)?;
    let pkg_manager = PackageManager::new(paths.clone());
    let mut found_anywhere = false;

    // Remove from declared-packages.yaml
    let decl_file = paths.modules_dir().join("declared-packages.yaml");
    if decl_file.exists() {
        let content = std::fs::read_to_string(&decl_file)?;
        let mut list: crate::config::PackageList = serde_yaml::from_str(&content).unwrap_or_default();
        let before = list.packages.len();
        list.packages.retain(|p| p.name() != package);
        if list.packages.len() != before {
            let yaml = serde_yaml::to_string(&list)?;
            std::fs::write(&decl_file, yaml)?;
            println!("  {} Removed '{}' from declared-packages.yaml", "→".blue(), package);
            found_anywhere = true;
        }
    }

    // Remove from system-packages-{host}/packages.yaml
    let sys_pkg_file = paths
        .modules_dir()
        .join(format!("system-packages-{}/packages.yaml", config.host));
    if sys_pkg_file.exists() {
        let content = std::fs::read_to_string(&sys_pkg_file)?;
        let mut list: crate::config::PackageList = serde_yaml::from_str(&content).unwrap_or_default();
        let before = list.packages.len();
        list.packages.retain(|p| p.name() != package);
        if list.packages.len() != before {
            let yaml = serde_yaml::to_string(&list)?;
            std::fs::write(&sys_pkg_file, yaml)?;
            println!("  {} Removed '{}' from system-packages", "→".blue(), package);
            found_anywhere = true;
        }
    }

    // Warn if found in other places (host config, modules)
    if config.packages.iter().any(|p| p.name() == package) {
        println!(
            "  {} '{}' is in your host config — remove it manually from {:?}",
            "!".yellow(),
            package,
            crate::config::resolve_config_path(paths)?
        );
        found_anywhere = true;
    }

    for module_name in &config.enabled_modules {
        if let Some(module_path) = pkg_manager.find_module_path(module_name) {
            if let Ok(module) = crate::config::load_module(&module_path) {
                if module.packages().iter().any(|p| p.name() == package) {
                    println!(
                        "  {} '{}' is in module '{}' — remove it manually from {:?}",
                        "!".yellow(),
                        package,
                        module_name,
                        module_path
                    );
                    found_anywhere = true;
                }
            }
        }
    }

    if found_anywhere {
        println!();
        println!(
            "{} '{}' will no longer be tracked by vcli. The package is still installed.",
            "✓".green(),
            package
        );
    } else {
        println!("{} '{}' was not found in any tracked config files.", "!".yellow(), package);
    }

    Ok(())
}


