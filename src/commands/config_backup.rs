use anyhow::{Context, Result};
use chrono::Utc;
use colored::*;
use std::io::{self, Write};

use crate::config::ConfigPaths;

pub fn save_config(paths: &ConfigPaths, label: &str, silent: bool) -> Result<()> {
    std::fs::create_dir_all(&paths.config_backups_dir)
        .context("Failed to create config backups directory")?;

    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let backup_name = format!("{}-{}", timestamp, label);
    let backup_path = paths.config_backups_dir.join(&backup_name);

    // Copy entire config dir (excluding state/ and .git/)
    copy_dir_filtered(&paths.config_dir, &backup_path)?;

    if !silent {
        println!("{} Config backed up to: {:?}", "✓".green(), backup_path);
    }

    // Enforce max_backups limit
    // Load config to get max_backups value — use default 5 if config unavailable
    let max_backups = crate::config::load_config(paths)
        .map(|c| c.config_backups.max_backups)
        .unwrap_or(5);

    if max_backups > 0 {
        prune_old_backups(&paths.config_backups_dir, max_backups as usize)?;
    }

    Ok(())
}

pub fn restore_config(paths: &ConfigPaths, backup: Option<String>, _json: bool) -> Result<()> {
    let backups = list_backups(paths)?;

    if backups.is_empty() {
        println!("{}", "No config backups found.".yellow());
        return Ok(());
    }

    let selected = if let Some(name) = backup {
        name
    } else if which::which("fzf").is_ok() {
        // Interactive fzf selection
        use std::io::Write as _;
        let list = backups.join("\n");
        let mut fzf = std::process::Command::new("fzf")
            .args(["--prompt", "Restore backup> "])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()?;

        if let Some(stdin) = fzf.stdin.take() {
            let mut stdin = stdin;
            let _ = stdin.write_all(list.as_bytes());
        }

        let output = fzf.wait_with_output()?;
        let sel = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if sel.is_empty() {
            return Ok(());
        }
        sel
    } else {
        println!("{}", "Available backups:".blue());
        for (i, b) in backups.iter().enumerate() {
            println!("  {}: {}", i + 1, b);
        }
        print!("Enter backup number to restore: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let n: usize = input.trim().parse().context("Invalid selection")?;
        backups
            .get(n - 1)
            .ok_or_else(|| anyhow::anyhow!("Invalid selection"))?
            .clone()
    };

    let backup_path = paths.config_backups_dir.join(&selected);
    if !backup_path.exists() {
        anyhow::bail!("Backup '{}' not found", selected);
    }

    println!("{}", format!("Restoring backup '{}'...", selected).blue());

    // Copy backup over config dir (excluding state/ and .git/)
    copy_dir_filtered(&backup_path, &paths.config_dir)?;

    println!("{}", "✓ Config restored. Run 'vcli sync' to apply.".green());
    Ok(())
}

fn list_backups(paths: &ConfigPaths) -> Result<Vec<String>> {
    if !paths.config_backups_dir.exists() {
        return Ok(Vec::new());
    }

    let mut backups: Vec<String> = std::fs::read_dir(&paths.config_backups_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    backups.sort();
    backups.reverse(); // Most recent first
    Ok(backups)
}

fn copy_dir_filtered(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip state/ and .git/ directories
        if name_str == "state" || name_str == ".git" {
            continue;
        }

        let dest_path = dst.join(&name);

        if path.is_dir() {
            copy_dir_filtered(&path, &dest_path)?;
        } else {
            std::fs::copy(&path, &dest_path)?;
        }
    }

    Ok(())
}

fn prune_old_backups(backups_dir: &std::path::Path, max_backups: usize) -> Result<()> {
    let mut backups: Vec<std::path::PathBuf> = std::fs::read_dir(backups_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.path())
        .collect();

    backups.sort();

    while backups.len() > max_backups {
        let oldest = backups.remove(0);
        let _ = std::fs::remove_dir_all(oldest);
    }

    Ok(())
}
