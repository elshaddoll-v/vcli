use anyhow::{Context, Result};
use colored::*;
use std::fs;

use crate::config::ConfigPaths;

pub fn run(paths: &ConfigPaths, bootstrap: bool) -> Result<()> {
    if paths.config_dir.exists() {
        println!("{}", "void-config directory already exists.".yellow());
        println!("Location: {}", paths.config_dir.display());
        return Ok(());
    }

    println!("{}", "Initializing void-config...".blue());

    // Create directory structure
    fs::create_dir_all(&paths.config_dir).context("Failed to create config directory")?;
    fs::create_dir_all(paths.config_dir.join("hosts")).context("Failed to create hosts dir")?;
    fs::create_dir_all(paths.config_dir.join("modules")).context("Failed to create modules dir")?;
    fs::create_dir_all(paths.config_dir.join("scripts")).context("Failed to create scripts dir")?;
    fs::create_dir_all(&paths.state_dir).context("Failed to create state directory")?;

    // Detect hostname
    let hostname = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "void".to_string());

    // Create pointer config.yaml
    let config_yaml = format!("host: {}\n", hostname);
    fs::write(&paths.config_file, &config_yaml).context("Failed to create config.yaml")?;

    // Create host file
    let host_file = paths.config_dir.join("hosts").join(format!("{}.yaml", hostname));
    let host_yaml = if bootstrap {
        bootstrap_host_yaml(&hostname)
    } else {
        minimal_host_yaml(&hostname)
    };
    fs::write(&host_file, host_yaml).context("Failed to create host file")?;

    // Create base.yaml
    let base_file = paths.config_dir.join("modules").join("base.yaml");
    let base_yaml = r#"description: Base system packages

packages:
  # Add your essential packages here
  - base-system
  - bash
  - curl
  - git
  - vim
"#;
    fs::write(&base_file, base_yaml).context("Failed to create base.yaml")?;

    // Create example module
    let example_module = paths.config_dir.join("modules").join("example.yaml");
    let example_yaml = r#"description: Example module - rename and customize

packages:
  - neovim
  - htop

# post_install_hook: scripts/setup-example.sh
# hook_behavior: once  # ask | always | once | skip
"#;
    fs::write(&example_module, example_yaml).context("Failed to create example.yaml")?;

    // Create .gitignore
    let gitignore = paths.config_dir.join(".gitignore");
    let gitignore_content = "state/config-backups/\n";
    fs::write(&gitignore, gitignore_content).context("Failed to create .gitignore")?;

    println!("{}", "✓ void-config initialized!".green());
    println!();
    println!("  Location:  {}", paths.config_dir.display());
    println!("  Host file: {}", host_file.display());
    println!("  Base pkgs: {}", base_file.display());
    println!();
    println!("Next steps:");
    println!("  1. Edit {} to add your packages", host_file.display());
    println!("  2. Run {} to sync", "vcli sync".cyan());
    println!("  3. Run {} to see git setup", "vcli repo init".cyan());

    Ok(())
}

fn minimal_host_yaml(hostname: &str) -> String {
    format!(
        r#"host: {hostname}
description: My Void Linux machine

# Enable modules from ~/.config/void-config/modules/
enabled_modules:
  - example

# Host-specific packages
packages: []

# Packages to exclude from modules
exclude: []

# Runit services to enable/disable
services:
  enabled: []
  disabled: []

# Flatpak scope: user or system
flatpak_scope: user

# Remove packages not in config during sync (default: false)
auto_prune: false

# Module processing: parallel (default) or sequential
module_processing: parallel

# Config backups before sync
config_backups:
  enabled: true
  max_backups: 5

# Hooks for pre/post update
# update_hooks:
#   pre_update: scripts/pre-update.sh
#   post_update: scripts/post-update.sh
#   behavior: ask
"#,
        hostname = hostname
    )
}

fn bootstrap_host_yaml(hostname: &str) -> String {
    format!(
        r#"host: {hostname}
description: My Void Linux machine

enabled_modules:
  - base
  - desktop
  - development

packages:
  - firefox
  - alacritty

exclude: []

services:
  enabled:
    - dbus
    - NetworkManager
  disabled: []

flatpak_scope: user
auto_prune: false
module_processing: parallel

config_backups:
  enabled: true
  max_backups: 5

default_apps:
  browser: firefox
  text_editor: nvim
  terminal: alacritty
"#,
        hostname = hostname
    )
}
