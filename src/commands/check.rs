//! vcli check — validate all declared package names against xbps repos before syncing.
use anyhow::Result;
use colored::*;

use crate::config::{load_config, ConfigPaths, PackageType};
use crate::package::PackageManager;

pub fn run(paths: &ConfigPaths) -> Result<()> {
    let config = load_config(paths)?;
    let pkg_manager = PackageManager::new(paths.clone());
    let declared = pkg_manager.get_declared_packages(&config)?;

    let xbps_pkgs: Vec<_> = declared.iter()
        .filter(|p| p.package_type == PackageType::Xbps)
        .collect();

    if xbps_pkgs.is_empty() {
        println!("{}", "No xbps packages declared.".yellow());
        return Ok(());
    }

    println!("{}", format!("Checking {} declared xbps packages...", xbps_pkgs.len()).blue());
    println!();

    let mut ok = 0usize;
    let mut bad: Vec<(String, Vec<String>)> = Vec::new();

    for pkg in &xbps_pkgs {
        if pkg_manager.xbps_pkg_exists(&pkg.name) {
            ok += 1;
            println!("  {} {}", "✓".green(), pkg.name);
        } else {
            // Search for suggestions
            let suggestions = find_suggestions(&pkg.name);
            bad.push((pkg.name.clone(), suggestions));
            println!("  {} {} — not found in repos", "✗".red(), pkg.name);
        }
    }

    println!();

    if bad.is_empty() {
        println!("{}", format!("✓ All {} packages are valid. Safe to sync.", ok).green());
    } else {
        println!("{}", format!("✗ {} package(s) not found in repos:", bad.len()).red());
        println!();
        for (pkg, suggestions) in &bad {
            println!("  {} {}", "→".blue(), pkg.bold());
            if !suggestions.is_empty() {
                println!("    Did you mean: {}", suggestions.join("  ").cyan());
            } else {
                println!("    Search: {}", format!("xbps-query -Rs {}", pkg).cyan());
            }
        }
        println!();
        println!("Fix the package names in your config, then run {} again.", "vcli sync".cyan());
    }

    Ok(())
}

fn find_suggestions(name: &str) -> Vec<String> {
    let output = std::process::Command::new("xbps-query")
        .args(["-Rs", name])
        .output();
    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.lines().filter_map(|line| {
                let line = line.trim();
                let rest = if line.starts_with("[*]") || line.starts_with("[-]") {
                    line[3..].trim()
                } else { line };
                let pkgver = rest.split_whitespace().next()?;
                let (pkg_name, _) = crate::package::split_xbps_pkgver(pkgver);
                if pkg_name.is_empty() { None } else { Some(pkg_name) }
            }).take(5).collect()
        }
        _ => Vec::new(),
    }
}
