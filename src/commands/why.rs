//! vcli why <pkg> — show what depends on a package and why it's installed.
use anyhow::Result;
use colored::*;

pub fn run(package: &str) -> Result<()> {
    println!("{}", format!("Why is '{}' installed?", package).blue().bold());
    println!();

    // Check if installed at all
    let query = std::process::Command::new("xbps-query")
        .args(["-p", "pkgver,short_desc,install-date,automatic-install", package])
        .output()?;

    if !query.status.success() {
        println!("{} '{}' is not installed.", "✗".red(), package);
        return Ok(());
    }

    // Show basic info
    let info = String::from_utf8_lossy(&query.stdout);
    for line in info.lines() {
        println!("  {}", line.dimmed());
    }
    println!();

    // Show reverse dependencies (what needs this package)
    let rdeps = std::process::Command::new("xbps-query")
        .args(["-X", package])
        .output()?;

    let rdeps_out = String::from_utf8_lossy(&rdeps.stdout);
    let rdep_lines: Vec<&str> = rdeps_out.lines().filter(|l| !l.trim().is_empty()).collect();

    if rdep_lines.is_empty() {
        println!("{}", "  No other packages depend on this.".dimmed());
        println!("  It may be safe to remove with: {}", format!("vcli remove {}", package).cyan());
    } else {
        println!(
            "{}",
            format!("  Required by {} package(s):", rdep_lines.len()).yellow()
        );
        for dep in rdep_lines.iter().take(20) {
            let dep = dep.trim();
            let (name, _) = crate::package::split_xbps_pkgver(dep);
            println!("    {} {}", "←".blue(), name.bold());
        }
        if rdep_lines.len() > 20 {
            println!("    ... and {} more", rdep_lines.len() - 20);
        }
    }

    println!();

    // Show what this package itself depends on
    let deps = std::process::Command::new("xbps-query")
        .args(["-x", package])
        .output()?;

    let deps_out = String::from_utf8_lossy(&deps.stdout);
    let dep_lines: Vec<&str> = deps_out.lines().filter(|l| !l.trim().is_empty()).collect();

    if !dep_lines.is_empty() {
        println!(
            "{}",
            format!("  Depends on {} package(s):", dep_lines.len()).blue()
        );
        for dep in dep_lines.iter().take(15) {
            println!("    {} {}", "→".dimmed(), dep.trim().dimmed());
        }
        if dep_lines.len() > 15 {
            println!("    ... and {} more", dep_lines.len() - 15);
        }
    }

    Ok(())
}
