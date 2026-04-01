use anyhow::Result;
use colored::*;
use std::io::{self, Write};

use crate::config::{load_config, resolve_editor, ConfigPaths};

pub fn run(paths: &ConfigPaths) -> Result<()> {
    let config = load_config(paths)?;
    let editor = resolve_editor(&config)?;

    // Collect all config files
    let mut files: Vec<std::path::PathBuf> = Vec::new();

    // Host config
    let config_path = crate::config::resolve_config_path(paths)?;
    files.push(config_path.clone());

    // Base module
    let base = paths.base_packages_file();
    if base.exists() {
        files.push(base);
    }

    // All module files
    let modules_dir = paths.modules_dir();
    if modules_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&modules_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map(|e| e == "yaml").unwrap_or(false) {
                    files.push(path);
                }
            }
        }
    }

    // Use fzf if available
    if which::which("fzf").is_ok() {
        let list = files
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("\n");

        use std::io::Write as _;
        let mut fzf = std::process::Command::new("fzf")
            .args(["--prompt", "Edit file> "])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()?;

        if let Some(stdin) = fzf.stdin.take() {
            let mut stdin = stdin;
            let _ = stdin.write_all(list.as_bytes());
        }

        let output = fzf.wait_with_output()?;
        let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if selected.is_empty() {
            return Ok(());
        }

        std::process::Command::new(&editor)
            .arg(&selected)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()?;
    } else {
        // Fallback: show list and prompt
        println!("{}", "Config files:".blue().bold());
        for (i, f) in files.iter().enumerate() {
            println!("  {}: {}", i + 1, f.display());
        }
        println!();
        print!("Enter number to edit (or path): ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        let path = if let Ok(n) = input.parse::<usize>() {
            if n >= 1 && n <= files.len() {
                files[n - 1].clone()
            } else {
                anyhow::bail!("Invalid selection");
            }
        } else {
            std::path::PathBuf::from(input)
        };

        std::process::Command::new(&editor)
            .arg(path)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()?;
    }

    Ok(())
}
