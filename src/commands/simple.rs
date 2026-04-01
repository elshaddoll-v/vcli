use anyhow::Result;
use colored::*;

use crate::config::{ConfigPaths, PackageEntry, PackageList};
use crate::package::PackageManager;

/// Install one or more packages and track them in declared-packages.yaml
pub fn install(package: &str, paths: &ConfigPaths) -> Result<()> {
    // Check the package exists in repos before trying to install
    let pkg_manager = PackageManager::new(paths.clone());
    if !pkg_manager.xbps_pkg_exists(package) {
        // Search for close matches and suggest them
        let suggestions = find_suggestions(package);
        eprintln!("{} Package '{}' not found in xbps repos.", "✗".red(), package);
        if !suggestions.is_empty() {
            eprintln!("{} Did you mean:", "?".yellow());
            for s in &suggestions {
                eprintln!("    {}", s.cyan());
            }
        } else {
            eprintln!("  Search with: {}", format!("xbps-query -Rs {}", package).cyan());
        }
        anyhow::bail!("package not found: {}", package);
    }

    println!("{}", format!("Installing {}...", package).blue());
    pkg_manager.install_xbps(&[package.to_string()], false)?;
    add_to_declared(package, paths)?;
    println!("{}", format!("✓ {} installed and tracked in config", package).green());
    Ok(())
}

/// Remove one or more packages and drop them from declared-packages.yaml
pub fn remove(package: &str, paths: &ConfigPaths) -> Result<()> {
    println!("{}", format!("Removing {}...", package).blue());

    // Remove from declared-packages.yaml
    let decl_file = paths.modules_dir().join("declared-packages.yaml");
    if decl_file.exists() {
        let content = std::fs::read_to_string(&decl_file)?;
        let mut list: PackageList = serde_yaml::from_str(&content).unwrap_or_default();
        let before = list.packages.len();
        list.packages.retain(|p| p.name() != package);
        if list.packages.len() != before {
            let yaml = serde_yaml::to_string(&list)?;
            std::fs::write(&decl_file, yaml)?;
            println!("  {} Removed '{}' from declared-packages.yaml", "→".blue(), package);
        }
    }

    let pkg_manager = PackageManager::new(paths.clone());
    pkg_manager.remove_xbps(&[package.to_string()], false)?;
    println!("{}", format!("✓ {} removed", package).green());
    Ok(())
}

fn add_to_declared(package: &str, paths: &ConfigPaths) -> Result<()> {
    let decl_file = paths.modules_dir().join("declared-packages.yaml");
    std::fs::create_dir_all(paths.modules_dir())?;

    let mut list: PackageList = if decl_file.exists() {
        let content = std::fs::read_to_string(&decl_file)?;
        serde_yaml::from_str(&content).unwrap_or_default()
    } else {
        PackageList {
            description: "Packages installed via vcli install".to_string(),
            ..Default::default()
        }
    };

    if list.packages.iter().any(|p| p.name() == package) {
        return Ok(());
    }

    list.packages.push(PackageEntry::Simple(package.to_string()));
    let yaml = serde_yaml::to_string(&list)?;
    std::fs::write(&decl_file, yaml)?;
    println!("  {} Tracked '{}' in declared-packages.yaml", "→".blue(), package);
    Ok(())
}

/// Search xbps repos for packages similar to the given name
fn find_suggestions(name: &str) -> Vec<String> {
    let output = std::process::Command::new("xbps-query")
        .args(["-Rs", name])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout
                .lines()
                .filter_map(|line| {
                    let line = line.trim();
                    let rest = if line.starts_with("[*]") || line.starts_with("[-]") {
                        line[3..].trim()
                    } else {
                        line
                    };
                    let pkgver = rest.split_whitespace().next()?;
                    let (pkg_name, _) = crate::package::split_xbps_pkgver(pkgver);
                    Some(pkg_name)
                })
                .take(5)
                .collect()
        }
        _ => Vec::new(),
    }
}
