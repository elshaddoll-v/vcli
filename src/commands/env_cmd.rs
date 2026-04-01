//! vcli env — manage central environment variables in ~/.config/environment.sh
//! This file is sourced by ~/.xprofile and fish/zsh configs.
//! Changing TERMINAL here changes it everywhere instantly.
use anyhow::Result;
use colored::*;
use std::path::PathBuf;

fn env_file() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    PathBuf::from(home).join(".config/environment.sh")
}

pub fn get(key: Option<&str>) -> Result<()> {
    let path = env_file();
    if !path.exists() {
        println!("{}", "No environment file found. Run 'vcli env init' first.".yellow());
        return Ok(());
    }

    let content = std::fs::read_to_string(&path)?;

    match key {
        None => {
            // Show all
            println!("{}", "Environment variables:".blue().bold());
            println!();
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') { continue; }
                if let Some(stripped) = line.strip_prefix("export ") {
                    let parts: Vec<&str> = stripped.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        let k = parts[0].bold();
                        let v = parts[1].trim_matches('"').cyan();
                        println!("  {} = {}", k, v);
                    }
                }
            }
        }
        Some(key) => {
            // Show one
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with(&format!("export {}=", key)) {
                    let val = line.split('=').nth(1).unwrap_or("").trim_matches('"');
                    println!("{}", val);
                    return Ok(());
                }
            }
            println!("{} '{}' not set", "!".yellow(), key);
        }
    }
    Ok(())
}

pub fn set(key: &str, value: &str) -> Result<()> {
    let path = env_file();

    let content = if path.exists() {
        std::fs::read_to_string(&path)?
    } else {
        String::new()
    };

    let new_line = format!("export {}=\"{}\"", key, value);
    let search = format!("export {}=", key);

    let mut found = false;
    let mut new_lines: Vec<String> = content.lines().map(|l| {
        if l.trim().starts_with(&search) {
            found = true;
            new_line.clone()
        } else {
            l.to_string()
        }
    }).collect();

    if !found {
        new_lines.push(new_line.clone());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(&path, new_lines.join("\n") + "\n")?;

    println!("{} {} = {}", "✓".green(), key.bold(), value.cyan());
    println!("  Restart your shell or run {} to apply", "source ~/.config/environment.sh".cyan());
    Ok(())
}

pub fn unset(key: &str) -> Result<()> {
    let path = env_file();
    if !path.exists() { return Ok(()); }

    let content = std::fs::read_to_string(&path)?;
    let search = format!("export {}=", key);

    let new_content: String = content.lines()
        .filter(|l| !l.trim().starts_with(&search))
        .map(|l| l.to_string() + "\n")
        .collect();

    std::fs::write(&path, new_content)?;
    println!("{} Unset '{}'", "✓".green(), key);
    Ok(())
}

pub fn init() -> Result<()> {
    let path = env_file();

    if path.exists() {
        println!("{}", "Environment file already exists.".yellow());
        println!("  Edit it at: {}", path.display().to_string().cyan());
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let template = r#"# vcli central environment — edit this file to change apps system-wide
# Source this from ~/.xprofile, ~/.bashrc, ~/.zshrc, ~/.config/fish/config.fish

# Core apps — change these to switch tools everywhere
export TERMINAL="alacritty"
export BROWSER="firefox"
export EDITOR="nvim"
export VISUAL="nvim"
export FILE_MANAGER="yazi"
export LAUNCHER="rofi -show drun"

# Shell
export SHELL="/bin/fish"

# Fonts
export FONT="JetBrainsMono Nerd Font"
export FONT_SIZE="12"

# Theme
export GTK_THEME="Adwaita-dark"
export ICON_THEME="Papirus-Dark"

# XDG
export XDG_CONFIG_HOME="$HOME/.config"
export XDG_DATA_HOME="$HOME/.local/share"
export XDG_CACHE_HOME="$HOME/.cache"

# Tools
export PAGER="bat"
export MANPAGER="sh -c 'col -bx | bat -l man -p'"

# PATH additions
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
"#;

    std::fs::write(&path, template)?;

    println!("{}", "✓ Environment file created!".green());
    println!("  Location: {}", path.display().to_string().cyan());
    println!();
    println!("Add to your shell config:");
    println!("  {}", "fish: source ~/.config/environment.sh".dimmed());
    println!("  {}", "bash/zsh: source ~/.config/environment.sh".dimmed());
    println!("  {}", "x11: source ~/.config/environment.sh in ~/.xprofile".dimmed());
    Ok(())
}

pub fn edit() -> Result<()> {
    let path = env_file();
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    std::process::Command::new(&editor)
        .arg(&path)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;
    Ok(())
}
