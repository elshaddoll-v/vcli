//! vcli modules — manage built-in module library.
//! Built-in modules ship with vcli and cover WMs, DEs, shells, CLI tools, etc.
//! Use `vcli modules add <name>` to copy one into your void-config.
use anyhow::Result;
use colored::*;
use std::path::PathBuf;

use crate::config::ConfigPaths;

/// Find the built-in modules directory (ships alongside the vcli binary)
fn builtin_modules_dir() -> Option<PathBuf> {
    // Look relative to the vcli binary
    let exe = std::env::current_exe().ok()?;
    let exe_dir = exe.parent()?;

    // Check <binary-dir>/modules/ (installed alongside binary)
    let next_to_binary = exe_dir.join("modules");
    if next_to_binary.exists() {
        return Some(next_to_binary);
    }

    // Check /usr/local/share/vcli/modules/
    let share = PathBuf::from("/usr/local/share/vcli/modules");
    if share.exists() {
        return Some(share);
    }

    // Check /usr/share/vcli/modules/
    let usr_share = PathBuf::from("/usr/share/vcli/modules");
    if usr_share.exists() {
        return Some(usr_share);
    }

    None
}

pub fn list() -> Result<()> {
    let dir = match builtin_modules_dir() {
        Some(d) => d,
        None => {
            println!("{}", "Built-in modules not found.".yellow());
            println!("Install vcli from the repo — modules ship with it.");
            println!("Expected location: /usr/local/share/vcli/modules/");
            return Ok(());
        }
    };

    println!("{}", "Built-in module library:".blue().bold());
    println!("  Use {} to add one to your config", "vcli modules add <name>".cyan());
    println!();

    // Group by prefix
    let categories = [
        ("wm-",      "Window Managers"),
        ("de-",      "Desktop Environments"),
        ("dm-",      "Display Managers"),
        ("x11-",     "X11"),
        ("wayland-", "Wayland"),
        ("shell-",   "Shells"),
        ("audio-",   "Audio"),
        ("dev-",     "Development"),
        ("system-",  "System"),
        ("",         "Other"),
    ];

    let mut entries: Vec<_> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "yaml").unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().trim_end_matches(".yaml").to_string())
        .collect();
    entries.sort();

    let mut printed: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (prefix, label) in &categories {
        let group: Vec<_> = entries.iter()
            .filter(|n| {
                if prefix.is_empty() {
                    !printed.contains(*n)
                } else {
                    n.starts_with(prefix)
                }
            })
            .cloned()
            .collect();

        if group.is_empty() { continue; }

        println!("  {}:", label.bold());
        for name in &group {
            // Read description from yaml
            let path = dir.join(format!("{}.yaml", name));
            let desc = read_description(&path).unwrap_or_default();
            let short_name = if prefix.is_empty() { name.clone() } else {
                name.trim_start_matches(prefix).to_string()
            };
            println!("    {:<25} {}", name.cyan(), desc.dimmed());
            printed.insert(name.clone());
        }
        println!();
    }

    Ok(())
}

pub fn add(paths: &ConfigPaths, name: &str) -> Result<()> {
    let dir = match builtin_modules_dir() {
        Some(d) => d,
        None => {
            anyhow::bail!("Built-in modules not found. Expected at /usr/local/share/vcli/modules/");
        }
    };

    // Support partial names — wm-i3 or just i3
    let full_name = if dir.join(format!("{}.yaml", name)).exists() {
        name.to_string()
    } else {
        // Search with prefix
        let prefixes = ["wm-", "de-", "dm-", "shell-", "audio-", "dev-", "system-",
                        "x11-", "wayland-"];
        let mut found = None;
        for prefix in &prefixes {
            let candidate = format!("{}{}", prefix, name);
            if dir.join(format!("{}.yaml", candidate)).exists() {
                found = Some(candidate);
                break;
            }
        }
        found.ok_or_else(|| anyhow::anyhow!(
            "Module '{}' not found in built-in library.\nRun 'vcli modules list' to see available modules.",
            name
        ))?
    };

    let source = dir.join(format!("{}.yaml", full_name));
    let dest_dir = paths.modules_dir();
    std::fs::create_dir_all(&dest_dir)?;
    let dest = dest_dir.join(format!("{}.yaml", full_name));

    if dest.exists() {
        println!("{} Module '{}' already exists in your config.", "!".yellow(), full_name);
        println!("  Location: {}", dest.display());
        return Ok(());
    }

    std::fs::copy(&source, &dest)?;

    println!("{} Added module '{}' to your config!", "✓".green(), full_name.cyan());
    println!("  Location: {}", dest.display());
    println!();
    println!("Enable it with: {}", format!("vcli module enable {}", full_name).cyan());
    println!("Then sync:      {}", "vcli sync".cyan());

    Ok(())
}

pub fn add_interactive(paths: &ConfigPaths) -> Result<()> {
    if which::which("fzf").is_err() {
        println!("{}", "fzf not installed. Use: vcli modules add <name>".yellow());
        return list();
    }

    let dir = match builtin_modules_dir() {
        Some(d) => d,
        None => anyhow::bail!("Built-in modules not found"),
    };

    let mut entries: Vec<String> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "yaml").unwrap_or(false))
        .map(|e| {
            let name = e.file_name().to_string_lossy()
                .trim_end_matches(".yaml").to_string();
            let path = e.path();
            let desc = read_description(&path).unwrap_or_default();
            format!("{:<25} {}", name, desc)
        })
        .collect();
    entries.sort();

    use std::io::Write;
    let input = entries.join("\n");
    let mut fzf = std::process::Command::new("fzf")
        .args(["--multi", "--prompt", "Add module> ",
               "--header", "TAB to select multiple, ENTER to add"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    if let Some(stdin) = fzf.stdin.take() {
        let mut s = stdin;
        let _ = s.write_all(input.as_bytes());
    }

    let out = fzf.wait_with_output()?;
    let selected = String::from_utf8_lossy(&out.stdout);

    for line in selected.lines() {
        let name = line.split_whitespace().next().unwrap_or("").trim();
        if !name.is_empty() {
            add(paths, name)?;
        }
    }

    Ok(())
}

fn read_description(path: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        if let Some(desc) = line.strip_prefix("description:") {
            return Some(desc.trim().trim_matches('"').to_string());
        }
    }
    None
}
