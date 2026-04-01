//! vcli rice — manage desktop rices (complete WM dotfile configurations).
//!
//! Design rules (fix for v5):
//! - rice apply only touches dotfiles, never packages
//! - vcli sync only touches packages/services, never dotfiles
//! - Conflicts are detected and reported clearly before anything is changed
//! - Every overwritten file is backed up to <file>.vcli-bak
//! - `vcli rice remove` restores backups
use anyhow::Result;
use colored::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::config::ConfigPaths;
use crate::dotfiles::{expand_tilde, link, unlink, LinkResult, UnlinkResult};

#[derive(Debug, Serialize, Deserialize)]
pub struct RiceManifest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_wm")]
    pub wm: String,
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub wallpaper: Option<String>,
    /// Dotfile symlinks: source (relative to rice dir) → target (~ expanded)
    #[serde(default)]
    pub symlinks: Vec<SymlinkEntry>,
    /// xbps packages this rice needs (installed by `vcli rice apply --with-packages`)
    #[serde(default)]
    pub packages: Vec<String>,
}

fn default_wm() -> String { "i3".to_string() }

#[derive(Debug, Serialize, Deserialize)]
pub struct SymlinkEntry {
    pub source: String,
    pub target: String,
}

fn rices_dir(paths: &ConfigPaths) -> PathBuf {
    paths.config_dir.join("rices")
}

fn current_rice_file(paths: &ConfigPaths) -> PathBuf {
    paths.state_dir.join("current-rice")
}

fn symlinks_state_file(paths: &ConfigPaths) -> PathBuf {
    paths.state_dir.join("rice-symlinks.yaml")
}

/// Load the list of symlinks currently managed by vcli rice
fn load_managed_symlinks(paths: &ConfigPaths) -> Vec<String> {
    let file = symlinks_state_file(paths);
    if !file.exists() { return Vec::new(); }
    std::fs::read_to_string(&file)
        .ok()
        .and_then(|c| serde_yaml::from_str::<Vec<String>>(&c).ok())
        .unwrap_or_default()
}

fn save_managed_symlinks(paths: &ConfigPaths, symlinks: &[String]) -> Result<()> {
    let file = symlinks_state_file(paths);
    std::fs::create_dir_all(file.parent().unwrap())?;
    let yaml = serde_yaml::to_string(symlinks)?;
    std::fs::write(&file, yaml)?;
    Ok(())
}

pub fn list(paths: &ConfigPaths) -> Result<()> {
    let dir = rices_dir(paths);
    if !dir.exists() {
        println!("{}", "No rices yet. Create one with: vcli rice create <name>".yellow());
        return Ok(());
    }

    let current = std::fs::read_to_string(current_rice_file(paths))
        .unwrap_or_default();
    let current = current.trim();

    println!("{}", "Available rices:".blue().bold());
    println!();

    let mut found = false;
    let mut entries: Vec<_> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.is_empty() || name.starts_with('.') { continue; }
        found = true;

        let is_active = name == current;
        let dot = if is_active { "●".green() } else { "○".dimmed() };
        let active_tag = if is_active { " [active]".green().to_string() } else { String::new() };

        match load_manifest(&path) {
            Ok(m) => {
                println!("  {} {}{}", dot, name.bold(), active_tag);
                println!("     {} · {} symlink(s) · packages: {}",
                    m.wm.cyan(),
                    m.symlinks.len(),
                    if m.packages.is_empty() { "none".dimmed().to_string() }
                    else { m.packages.len().to_string() }
                );
            }
            Err(_) => println!("  {} {} {} (no manifest.toml)", dot, name.bold(), active_tag),
        }
    }

    if !found {
        println!("  {}", "(no rices yet)".dimmed());
    }
    Ok(())
}

/// Apply a rice — only touches dotfiles, never packages.
/// Use --with-packages to also install required packages.
pub fn apply(paths: &ConfigPaths, name: &str, dry_run: bool, force: bool) -> Result<()> {
    let rice_dir = rices_dir(paths).join(name);
    if !rice_dir.exists() {
        anyhow::bail!(
            "Rice '{}' not found.\nAvailable: vcli rice list\nCreate new: vcli rice create {}",
            name, name
        );
    }

    apply_dots(paths, Some(name), dry_run, force)
}

/// Apply only the dotfiles of a rice (or current rice if name is None).
/// This is the single source of truth for dotfile symlinking.
pub fn apply_dots(paths: &ConfigPaths, name: Option<&str>, dry_run: bool, force: bool) -> Result<()> {
    // Resolve which rice to use
    let rice_name = match name {
        Some(n) => n.to_string(),
        None => {
            let file = current_rice_file(paths);
            if !file.exists() {
                anyhow::bail!("No active rice. Specify a name: vcli dots <rice-name>");
            }
            std::fs::read_to_string(&file)?.trim().to_string()
        }
    };

    let rice_dir = rices_dir(paths).join(&rice_name);
    if !rice_dir.exists() {
        anyhow::bail!("Rice '{}' not found", rice_name);
    }

    let manifest = load_manifest(&rice_dir)?;
    let home = std::env::var("HOME").unwrap_or_default();

    println!("{}", format!("Applying dotfiles for rice '{}'...", rice_name).blue().bold());
    println!("  WM: {}  Symlinks: {}",
        manifest.wm.cyan(),
        manifest.symlinks.len()
    );
    println!();

    if manifest.symlinks.is_empty() {
        println!("{}", "No symlinks defined in manifest.toml".yellow());
        println!("Add [[symlinks]] entries to {}/manifest.toml", rice_dir.display());
        return Ok(());
    }

    // Phase 1: Detect conflicts BEFORE touching anything
    let mut conflicts: Vec<(String, PathBuf)> = Vec::new();
    for entry in &manifest.symlinks {
        let source = rice_dir.join(&entry.source);
        let target = PathBuf::from(expand_tilde(&entry.target));
        if (target.exists() || target.is_symlink()) && !target.is_symlink() && !force {
            conflicts.push((entry.target.clone(), target.clone()));
        }
    }

    if !conflicts.is_empty() && !force {
        println!("{}", format!("✗ {} conflict(s) found — nothing was changed:", conflicts.len()).red().bold());
        println!();
        for (target_str, target_path) in &conflicts {
            let is_dir = target_path.is_dir();
            println!("  {} {} {}",
                "→".red(),
                target_str,
                if is_dir { "(directory)" } else { "(file)" }
            );
        }
        println!();
        println!("Options:");
        println!("  {} — backs up conflicting files to <file>.vcli-bak and links",
            "vcli dots --force".cyan());
        println!("  {} — manually move/remove the conflicting files first",
            "vcli dots".cyan());
        return Ok(());
    }

    // Phase 2: Apply symlinks
    let mut linked = 0usize;
    let mut skipped = 0usize;
    let mut backed_up = 0usize;
    let mut missing = 0usize;
    let mut managed: Vec<String> = Vec::new();

    for entry in &manifest.symlinks {
        let source = rice_dir.join(&entry.source);
        let target_str = expand_tilde(&entry.target);
        let target = PathBuf::from(&target_str);

        match link(&source, &target, force, dry_run)? {
            LinkResult::Linked => {
                println!("  {} {} → {}", "✓".green(), entry.target, entry.source);
                linked += 1;
                managed.push(target_str.clone());
            }
            LinkResult::AlreadyLinked => {
                println!("  {} {} (already linked)", "·".dimmed(), entry.target);
                skipped += 1;
                managed.push(target_str.clone());
            }
            LinkResult::WouldLink => {
                println!("  {} would link: {} → {}", "→".blue(), entry.target, entry.source);
                linked += 1;
            }
            LinkResult::WouldBackup => {
                println!("  {} would backup + link: {}", "→".yellow(), entry.target);
                backed_up += 1;
                linked += 1;
            }
            LinkResult::Conflict => {
                // Shouldn't reach here — caught in phase 1
                println!("  {} conflict: {} (skipped)", "!".yellow(), entry.target);
                skipped += 1;
            }
            LinkResult::SourceMissing => {
                println!("  {} source not found: {}/{}", "✗".red(), entry.source, entry.source);
                missing += 1;
            }
        }
    }

    if !dry_run {
        // Save managed symlinks state
        save_managed_symlinks(paths, &managed)?;

        // Save current rice
        std::fs::create_dir_all(&paths.state_dir)?;
        std::fs::write(current_rice_file(paths), &rice_name)?;

        // Apply wallpaper/theme if configured
        if let Some(ref wallpaper) = manifest.wallpaper {
            let wp: String = if wallpaper.starts_with('/') || wallpaper.starts_with('~') {
                wallpaper.replace('~', &home)
            } else {
                rice_dir.join(wallpaper).to_string_lossy().to_string()
            };
            println!();
            let _ = crate::commands::theme::apply(Some(&wp), false);
        }
    }

    println!();
    if dry_run {
        println!("{}", "Dry run — no changes made.".blue());
        println!("  Would link: {}  back up: {}  missing: {}", linked, backed_up, missing);
    } else {
        let mut parts = Vec::new();
        if linked > 0 { parts.push(format!("{} linked", linked)); }
        if backed_up > 0 { parts.push(format!("{} backed up", backed_up)); }
        if skipped > 0 { parts.push(format!("{} already correct", skipped)); }
        if missing > 0 { parts.push(format!("{} sources missing", missing)); }
        println!("{}", format!("✓ Done — {}", parts.join(", ")).green());

        if manifest.packages.is_empty() {
            // nothing
        } else {
            println!();
            println!("  This rice requires {} package(s).", manifest.packages.len());
            println!("  Install them with: {}", "vcli sync".cyan());
            println!("  (Add the rice's packages to your host config or a module first)");
        }
    }

    Ok(())
}

/// Remove rice symlinks, restoring backups where they exist.
pub fn remove(paths: &ConfigPaths, name: &str) -> Result<()> {
    let rice_dir = rices_dir(paths).join(name);
    if !rice_dir.exists() {
        anyhow::bail!("Rice '{}' not found", name);
    }

    let manifest = load_manifest(&rice_dir)?;
    println!("{}", format!("Removing rice '{}' symlinks...", name).blue());
    println!();

    for entry in &manifest.symlinks {
        let target_str = expand_tilde(&entry.target);
        let target = PathBuf::from(&target_str);

        match unlink(&target)? {
            UnlinkResult::Removed => {
                println!("  {} removed: {}", "✓".green(), entry.target);
            }
            UnlinkResult::Restored => {
                println!("  {} removed + restored backup: {}", "✓".green(), entry.target);
            }
            UnlinkResult::NotASymlink => {
                println!("  {} not a symlink (skipped): {}", "·".dimmed(), entry.target);
            }
        }
    }

    // Clear current rice if it was this one
    let current = std::fs::read_to_string(current_rice_file(paths))
        .unwrap_or_default();
    if current.trim() == name {
        let _ = std::fs::remove_file(current_rice_file(paths));
    }

    // Clear managed symlinks state
    save_managed_symlinks(paths, &[])?;

    println!();
    println!("{}", format!("✓ Rice '{}' removed", name).green());
    Ok(())
}

pub fn create(paths: &ConfigPaths, name: &str) -> Result<()> {
    let rice_dir = rices_dir(paths).join(name);
    if rice_dir.exists() {
        anyhow::bail!("Rice '{}' already exists at {:?}", name, rice_dir);
    }

    std::fs::create_dir_all(rice_dir.join("config"))?;
    std::fs::create_dir_all(rice_dir.join("scripts"))?;

    let manifest = format!(r#"name = "{name}"
description = "My {name} rice"
wm = "i3"
# theme = "catppuccin-mocha"
# wallpaper = "~/Pictures/wallpapers/{name}.jpg"

# Packages this rice needs — add them to your host config or a module,
# then run `vcli sync` to install. Rice apply never installs packages.
packages = [
    "i3",
    "picom",
    "rofi",
    "dunst",
    "alacritty",
]

# Dotfile symlinks
# source: path relative to this rice directory
# target: path on your system (~ is expanded to $HOME)
[[symlinks]]
source = "config/i3"
target = "~/.config/i3"

[[symlinks]]
source = "config/rofi"
target = "~/.config/rofi"

[[symlinks]]
source = "config/alacritty"
target = "~/.config/alacritty"

[[symlinks]]
source = "config/dunst"
target = "~/.config/dunst"

[[symlinks]]
source = "config/picom"
target = "~/.config/picom"
"#, name = name);

    std::fs::write(rice_dir.join("manifest.toml"), manifest)?;

    println!("{}", format!("✓ Rice '{}' created!", name).green());
    println!("  Location: {}", rice_dir.display().to_string().cyan());
    println!();
    println!("Steps:");
    println!("  1. Copy your dotfiles into {}/config/", rice_dir.display());
    println!("     e.g. cp -r ~/.config/i3 {}/config/", rice_dir.display());
    println!("  2. Edit {}/manifest.toml to adjust symlinks", rice_dir.display());
    println!("  3. Preview: {}", format!("vcli dots {} --dry-run", name).cyan());
    println!("  4. Apply:   {}", format!("vcli dots {}", name).cyan());
    Ok(())
}

pub fn current(paths: &ConfigPaths) -> Result<()> {
    let file = current_rice_file(paths);
    if file.exists() {
        let name = std::fs::read_to_string(&file)?;
        let name = name.trim();
        println!("Active rice: {}", name.cyan().bold());
        let rice_dir = rices_dir(paths).join(name);
        if let Ok(manifest) = load_manifest(&rice_dir) {
            println!("  WM: {}  Symlinks: {}", manifest.wm, manifest.symlinks.len());
        }
    } else {
        println!("{}", "No rice applied yet.".yellow());
        println!("Apply one with: {}", "vcli dots <rice-name>".cyan());
    }
    Ok(())
}

fn load_manifest(rice_dir: &Path) -> Result<RiceManifest> {
    let path = rice_dir.join("manifest.toml");
    let content = std::fs::read_to_string(&path)
        .map_err(|_| anyhow::anyhow!(
            "manifest.toml not found in {:?}\nCreate a rice with: vcli rice create <name>",
            rice_dir
        ))?;
    toml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse manifest.toml: {}\n\nFile: {:?}", e, path))
}
