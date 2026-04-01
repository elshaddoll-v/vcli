//! vcli orphans — packages installed as dependencies that nothing depends on anymore.
use anyhow::Result;
use colored::*;
use serde::Serialize;

use crate::config::{load_config, ConfigPaths};
use crate::package::PackageManager;

#[derive(Serialize)]
struct OrphanPackage {
    name: String,
    description: String,
    size: String,
}

pub fn run(paths: &ConfigPaths, remove: bool, json: bool) -> Result<()> {
    if !json {
        println!("{}", "Finding orphaned packages...".blue());
        println!();
    }

    // xbps-query -O lists packages installed automatically with no reverse deps
    let output = std::process::Command::new("xbps-query")
        .args(["-O"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut orphans: Vec<OrphanPackage> = Vec::new();

    for line in stdout.lines() {
        let pkgver = line.trim();
        if pkgver.is_empty() { continue; }
        let (name, _) = crate::package::split_xbps_pkgver(pkgver);
        if name.is_empty() { continue; }

        let desc = get_pkg_description(&name).unwrap_or_default();
        let size = get_pkg_size(&name).unwrap_or_else(|| "?".to_string());

        orphans.push(OrphanPackage { name, description: desc, size });
    }

    // Filter out packages that ARE declared in config (user wants them even if no rdeps)
    if paths.config_file.exists() {
        if let Ok(config) = load_config(paths) {
            let pkg_manager = PackageManager::new(paths.clone());
            if let Ok(declared) = pkg_manager.get_declared_packages(&config) {
                let declared_names: std::collections::HashSet<String> =
                    declared.iter().map(|p| p.name.clone()).collect();
                orphans.retain(|o| !declared_names.contains(&o.name));
            }
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&orphans)?);
        return Ok(());
    }

    if orphans.is_empty() {
        println!("{}", "✓ No orphaned packages found.".green());
        return Ok(());
    }

    println!(
        "{}",
        format!("{} orphaned package(s) (auto-installed deps, nothing needs them):", orphans.len())
            .yellow()
            .bold()
    );
    println!();

    let total_shown = orphans.len().min(50);
    for pkg in orphans.iter().take(total_shown) {
        let desc = if pkg.description.len() > 55 {
            format!("{}...", &pkg.description[..52])
        } else {
            pkg.description.clone()
        };
        println!(
            "  {} {}  {}  {}",
            "→".blue(),
            pkg.name.bold(),
            pkg.size.dimmed(),
            desc.dimmed()
        );
    }

    if orphans.len() > total_shown {
        println!("  ... and {} more", orphans.len() - total_shown);
    }

    println!();

    if remove {
        println!("{}", "Removing orphaned packages...".blue());
        let names: Vec<String> = orphans.iter().map(|o| o.name.clone()).collect();
        let pkg_manager = PackageManager::new(paths.clone());
        pkg_manager.remove_xbps(&names, false)?;
        println!("{}", format!("✓ Removed {} orphaned packages.", names.len()).green());
    } else {
        println!("To remove them:    {}", "vcli orphans --remove".cyan());
        println!("To keep one:       {}", "vcli install <name>  (tracks it in config)".cyan());
    }

    Ok(())
}

fn get_pkg_description(name: &str) -> Option<String> {
    let output = std::process::Command::new("xbps-query")
        .args(["-p", "short_desc", name])
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

fn get_pkg_size(name: &str) -> Option<String> {
    let output = std::process::Command::new("xbps-query")
        .args(["-p", "installed_size", name])
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}
