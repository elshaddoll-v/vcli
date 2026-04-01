//! vcli outdated — show packages that have updates available.
use anyhow::Result;
use colored::*;
use serde::Serialize;

#[derive(Serialize)]
struct OutdatedPackage {
    name: String,
    installed: String,
    available: String,
}

pub fn run(json: bool) -> Result<()> {
    if !json {
        println!("{}", "Checking for updates...".blue());
        println!();
    }

    // xbps-install -un lists packages with pending updates (dry-run update)
    let output = std::process::Command::new("xbps-install")
        .args(["-un"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut outdated: Vec<OutdatedPackage> = Vec::new();

    for line in stdout.lines() {
        // Output format: "update: pkgname-newver ..."
        // or just "pkgname-newver (update)"
        let line = line.trim();
        if line.is_empty() { continue; }

        if line.contains("update") || line.contains("upgrade") {
            // Parse: "pkgname-newversion update (pkgname-oldversion)"
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() >= 2 {
                let pkgver = parts[0];
                let (name, new_ver) = crate::package::split_xbps_pkgver(pkgver);

                // Get currently installed version
                let old_ver = get_installed_version(&name).unwrap_or_else(|| "?".to_string());

                outdated.push(OutdatedPackage {
                    name,
                    installed: old_ver,
                    available: new_ver,
                });
            }
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&outdated)?);
        return Ok(());
    }

    if outdated.is_empty() {
        println!("{}", "✓ Everything is up to date!".green());
        return Ok(());
    }

    println!("{}", format!("{} package(s) have updates available:", outdated.len()).yellow().bold());
    println!();

    // Find longest name for alignment
    let max_name = outdated.iter().map(|p| p.name.len()).max().unwrap_or(10);

    for pkg in &outdated {
        println!(
            "  {:<width$}  {} → {}",
            pkg.name.bold(),
            pkg.installed.dimmed(),
            pkg.available.green(),
            width = max_name
        );
    }

    println!();
    println!("Run {} to install all updates.", "vcli update".cyan());

    Ok(())
}

fn get_installed_version(pkg_name: &str) -> Option<String> {
    let output = std::process::Command::new("xbps-query")
        .args(["-p", "pkgver", pkg_name])
        .output()
        .ok()?;

    if output.status.success() {
        let ver_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let (_, version) = crate::package::split_xbps_pkgver(&ver_str);
        if !version.is_empty() { return Some(version); }
    }
    None
}
