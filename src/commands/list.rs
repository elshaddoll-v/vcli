//! vcli list — show all declared packages with their install status.
use anyhow::Result;
use colored::*;
use serde::Serialize;

use crate::config::{load_config, ConfigPaths, PackageType};
use crate::package::PackageManager;

#[derive(Serialize)]
struct PackageInfo {
    name: String,
    pkg_type: String,
    installed: bool,
    source: String,
}

pub fn run(paths: &ConfigPaths, json: bool) -> Result<()> {
    let config = load_config(paths)?;
    let pkg_manager = PackageManager::new(paths.clone());
    let declared = pkg_manager.get_declared_packages(&config)?;

    let installed_xbps = pkg_manager.get_installed_xbps_packages()
        .unwrap_or_default()
        .into_iter()
        .map(|(n, _)| n)
        .collect::<std::collections::HashSet<_>>();

    let installed_flatpaks = pkg_manager
        .get_installed_flatpaks(config.flatpak_scope.as_flag())
        .unwrap_or_default()
        .into_iter()
        .collect::<std::collections::HashSet<_>>();

    if json {
        let infos: Vec<PackageInfo> = declared.iter().map(|p| {
            let installed = match p.package_type {
                PackageType::Xbps => installed_xbps.contains(&p.name),
                PackageType::Flatpak => installed_flatpaks.contains(&p.name),
            };
            PackageInfo {
                name: p.name.clone(),
                pkg_type: format!("{:?}", p.package_type).to_lowercase(),
                installed,
                source: String::new(),
            }
        }).collect();
        println!("{}", serde_json::to_string_pretty(&infos)?);
        return Ok(());
    }

    let total = declared.len();
    let installed_count = declared.iter().filter(|p| match p.package_type {
        PackageType::Xbps => installed_xbps.contains(&p.name),
        PackageType::Flatpak => installed_flatpaks.contains(&p.name),
    }).count();

    println!("{}", format!("Declared packages ({}/{} installed):", installed_count, total).blue().bold());
    println!();

    for pkg in &declared {
        let is_installed = match pkg.package_type {
            PackageType::Xbps => installed_xbps.contains(&pkg.name),
            PackageType::Flatpak => installed_flatpaks.contains(&pkg.name),
        };
        let status = if is_installed { "✓".green() } else { "✗".red() };
        let type_tag = match pkg.package_type {
            PackageType::Xbps => "".to_string(),
            PackageType::Flatpak => " [flatpak]".dimmed().to_string(),
        };
        println!("  {} {}{}", status, pkg.name, type_tag);
    }

    println!();
    if installed_count < total {
        println!(
            "{} {} package(s) declared but not installed — run {}",
            "!".yellow(), total - installed_count, "vcli sync".cyan()
        );
    } else {
        println!("{}", "✓ All declared packages are installed.".green());
    }

    Ok(())
}
