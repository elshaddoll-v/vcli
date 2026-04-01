//! vcli doctor — full system health check (runs as root for complete info).
use anyhow::Result;
use colored::*;

use crate::config::{load_config, ConfigPaths, PackageType};
use crate::package::PackageManager;

pub fn run(paths: &ConfigPaths) -> Result<()> {
    println!("{}", "vcli doctor — system health check".blue().bold());
    println!();

    let mut issues = 0usize;
    let mut warnings = 0usize;

    // ── 1. Config ─────────────────────────────────────────────────────────────
    print!("  Config file            ");
    match load_config(paths) {
        Ok(config) => {
            println!("{}", "OK".green());

            // ── 2. Package names ─────────────────────────────────────────────
            print!("  Package names          ");
            let pkg_manager = PackageManager::new(paths.clone());
            if let Ok(declared) = pkg_manager.get_declared_packages(&config) {
                let xbps_pkgs: Vec<_> = declared.iter()
                    .filter(|p| p.package_type == PackageType::Xbps)
                    .collect();

                let mut bad: Vec<String> = Vec::new();
                for pkg in &xbps_pkgs {
                    if !pkg_manager.xbps_pkg_exists(&pkg.name) {
                        bad.push(pkg.name.clone());
                    }
                }

                if bad.is_empty() {
                    println!("{} ({} packages)", "OK".green(), xbps_pkgs.len());
                } else {
                    println!("{} ({} bad names)", "WARN".yellow(), bad.len());
                    for n in &bad {
                        println!("      {} {} — run {}", "!".yellow(), n, "vcli check".cyan());
                    }
                    warnings += bad.len();
                }

                // ── 3. Modules ───────────────────────────────────────────────
                print!("  Enabled modules        ");
                let missing: Vec<_> = config.enabled_modules.iter()
                    .filter(|m| pkg_manager.find_module_path(m).is_none())
                    .collect();

                if missing.is_empty() {
                    println!("{} ({} modules)", "OK".green(), config.enabled_modules.len());
                } else {
                    println!("{} ({} missing)", "WARN".yellow(), missing.len());
                    for m in &missing {
                        println!("      {} '{}' not found", "!".yellow(), m);
                    }
                    warnings += missing.len();
                }

                // ── 4. Not installed ─────────────────────────────────────────
                print!("  Packages installed     ");
                let installed = pkg_manager.get_installed_xbps_packages().unwrap_or_default();
                let inst_names: std::collections::HashSet<_> =
                    installed.iter().map(|(n,_)| n.clone()).collect();
                let not_installed: Vec<_> = xbps_pkgs.iter()
                    .filter(|p| !inst_names.contains(&p.name))
                    .collect();
                if not_installed.is_empty() {
                    println!("{}", "OK".green());
                } else {
                    println!("{} ({} not installed — run vcli sync)", "INFO".cyan(), not_installed.len());
                }
            }
        }
        Err(e) => {
            println!("{} — {}", "FAIL".red(), e);
            println!("      Run {} to create config", "vcli init".cyan());
            issues += 1;
        }
    }

    // ── 5. Broken deps (needs root — skip with warning if not root) ────────────
    print!("  Broken dependencies    ");
    if unsafe { libc::geteuid() } == 0 {
        let out = std::process::Command::new("xbps-pkgdb").args(["-a"]).output();
        match out {
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                let broken: Vec<_> = stderr.lines()
                    .filter(|l| l.contains("broken") || l.contains("MISSING"))
                    .collect();
                if broken.is_empty() {
                    println!("{}", "OK".green());
                } else {
                    println!("{} ({} issues)", "FAIL".red(), broken.len());
                    for l in broken.iter().take(3) {
                        println!("      {}", l.trim());
                    }
                    println!("      Fix: {}", "xbps-install -Su".cyan());
                    issues += broken.len();
                }
            }
            Err(_) => println!("{}", "SKIP".dimmed()),
        }
    } else {
        println!("{}", "SKIP (needs root — run: sudo -E vcli doctor)".dimmed());
    }

    // ── 6. Orphans ────────────────────────────────────────────────────────────
    print!("  Orphaned packages      ");
    let orphans = std::process::Command::new("xbps-query").args(["-O"]).output();
    match orphans {
        Ok(o) => {
            let count = String::from_utf8_lossy(&o.stdout)
                .lines().filter(|l| !l.trim().is_empty()).count();
            if count == 0 {
                println!("{}", "OK".green());
            } else {
                println!("{} ({} found — run vcli orphans)", "WARN".yellow(), count);
                warnings += 1;
            }
        }
        Err(_) => println!("{}", "SKIP".dimmed()),
    }

    // ── 7. Updates ────────────────────────────────────────────────────────────
    print!("  Pending updates        ");
    let updates = std::process::Command::new("xbps-install").args(["-un"]).output();
    match updates {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let count = stdout.lines()
                .filter(|l| !l.trim().is_empty() && !l.starts_with('['))
                .count();
            if count == 0 {
                println!("{}", "OK".green());
            } else {
                println!("{} ({} — run vcli update)", "INFO".cyan(), count);
            }
        }
        Err(_) => println!("{}", "SKIP".dimmed()),
    }

    // ── 8. Disk space ─────────────────────────────────────────────────────────
    print!("  Disk space (/)         ");
    let df = std::process::Command::new("df").args(["-h", "/"]).output();
    match df {
        Ok(o) => {
            let s = String::from_utf8_lossy(&o.stdout);
            if let Some(line) = s.lines().nth(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    let pct: u64 = parts[4].trim_end_matches('%').parse().unwrap_or(0);
                    let used = parts[2]; let total = parts[1];
                    if pct >= 90 {
                        println!("{} ({}/{} {}%)", "CRIT".red(), used, total, pct);
                        issues += 1;
                    } else if pct >= 75 {
                        println!("{} ({}/{} {}%)", "WARN".yellow(), used, total, pct);
                        warnings += 1;
                    } else {
                        println!("{} ({}/{} {}%)", "OK".green(), used, total, pct);
                    }
                }
            }
        }
        Err(_) => println!("{}", "SKIP".dimmed()),
    }

    // ── 9. sxhkd running? ─────────────────────────────────────────────────────
    print!("  sxhkd running          ");
    let sxhkd = std::process::Command::new("pgrep").args(["-x", "sxhkd"]).output()
        .map(|o| o.status.success()).unwrap_or(false);
    if sxhkd {
        println!("{}", "OK".green());
    } else {
        println!("{}", "Not running (start with: sxhkd &)".dimmed());
    }

    // ── 10. Desktop tools present? ────────────────────────────────────────────
    print!("  Desktop tools          ");
    let tools = ["fzf", "rofi", "dunst", "feh", "sxhkd"];
    let missing_tools: Vec<_> = tools.iter()
        .filter(|t| which::which(t).is_err())
        .collect();
    if missing_tools.is_empty() {
        println!("{}", "OK".green());
    } else {
        println!("{} (missing: {})", "WARN".yellow(),
            missing_tools.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", "));
        warnings += 1;
    }

    // ── 11. Color generator ───────────────────────────────────────────────────
    print!("  Color generator        ");
    if which::which("wallust").is_ok() {
        println!("{} (wallust)", "OK".green());
    } else if which::which("wal").is_ok() {
        println!("{} (pywal)", "OK".green());
    } else {
        println!("{}", "None (install: cargo install wallust OR vcli add python3-pywal)".yellow());
        warnings += 1;
    }

    // ── Summary ───────────────────────────────────────────────────────────────
    println!();
    println!("{}", "─".repeat(45).dimmed());
    if issues == 0 && warnings == 0 {
        println!("{}", "✓ Everything looks healthy!".green().bold());
    } else {
        if issues > 0 {
            println!("{}", format!("✗ {} issue(s) need attention", issues).red().bold());
        }
        if warnings > 0 {
            println!("{}", format!("⚠ {} warning(s)", warnings).yellow().bold());
        }
    }

    Ok(())
}
