use anyhow::Result;
use colored::*;
use std::io::{self, Write};

use crate::config::{load_config, ConfigPaths};

pub fn run(paths: &ConfigPaths, no_backup: bool, no_hooks: bool) -> Result<()> {
    let config = load_config(paths)?;

    // Run pre-update hook
    if !no_hooks {
        if let Some(ref hook) = config.update_hooks.pre_update {
            run_update_hook(paths, hook, &config.update_hooks.behavior, "pre-update", config.update_hooks.run_as_user)?;
        }
    }

    // Config backup
    if !no_backup && config.config_backups.enabled {
        println!("{}", "Creating configuration backup...".blue());
        match crate::commands::config_backup::save_config(paths, "auto-update", true) {
            Ok(_) => println!("{}", "✓ Configuration backup created".green()),
            Err(e) => println!("{}", format!("⚠ Warning: {}", e).yellow()),
        }
    }

    // Update with xbps-install -Su
    println!();
    println!("{}", "Updating system with xbps-install -Su...".blue());
    let status = std::process::Command::new("xbps-install")
        .args(["-Su"])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if !status.success() {
        anyhow::bail!("xbps-install -Su failed");
    }

    // Update flatpaks if any are declared
    let declared = crate::package::PackageManager::new(paths.clone())
        .get_declared_packages(&config)
        .unwrap_or_default();

    let has_flatpaks = declared
        .iter()
        .any(|p| p.package_type == crate::config::PackageType::Flatpak);

    if has_flatpaks {
        println!();
        println!("{}", "Updating flatpaks...".blue());
        let _ = std::process::Command::new("flatpak")
            .args(["update", "-y"])
            .status();
    }

    // Run post-update hook
    if !no_hooks {
        if let Some(ref hook) = config.update_hooks.post_update {
            run_update_hook(paths, hook, &config.update_hooks.behavior, "post-update", config.update_hooks.run_as_user)?;
        }
    }

    println!();
    println!("{}", "✓ System updated successfully".green());

    Ok(())
}

fn run_update_hook(
    paths: &ConfigPaths,
    hook: &str,
    behavior: &str,
    hook_type: &str,
    run_as_user: bool,
) -> Result<()> {
    let hook_path = paths.config_dir.join(hook);
    if !hook_path.exists() {
        return Ok(());
    }

    if behavior == "skip" {
        return Ok(());
    }

    if behavior == "ask" {
        println!();
        println!("{}", format!("{} hook: {}", hook_type, hook).blue().bold());
        print!("Run this hook? [Y/n] ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input.trim().eq_ignore_ascii_case("n") {
            println!("{}", "Skipping".yellow());
            return Ok(());
        }
    }

    println!("{}", format!("Running {}...", hook_type).blue());

    let status = if run_as_user {
        if let Ok(user) = std::env::var("USER") {
            std::process::Command::new("sudo")
                .args(["-u", &user, "bash", hook_path.to_str().unwrap()])
                .status()?
        } else {
            std::process::Command::new("bash")
                .arg(hook_path.to_str().unwrap())
                .status()?
        }
    } else {
        std::process::Command::new("bash")
            .arg(hook_path.to_str().unwrap())
            .status()?
    };

    if !status.success() {
        anyhow::bail!("{} hook failed", hook_type);
    }

    println!("{}", format!("✓ {} completed", hook_type).green());
    Ok(())
}
