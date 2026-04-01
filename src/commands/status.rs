use anyhow::Result;
use colored::*;
use serde::Serialize;

use crate::config::{load_config, ConfigPaths};
use crate::package::PackageManager;
use crate::services::ServiceManager;

#[derive(Serialize)]
struct StatusOutput {
    host: String,
    description: String,
    config_file: String,
    enabled_modules: Vec<String>,
    declared_packages: usize,
    installed_xbps: usize,
    services_enabled: Vec<String>,
    services_disabled: Vec<String>,
}

pub fn run(paths: &ConfigPaths, json: bool) -> Result<()> {
    let config = load_config(paths)?;
    let pkg_manager = PackageManager::new(paths.clone());

    let declared = pkg_manager.get_declared_packages(&config).unwrap_or_default();
    let installed_xbps = pkg_manager.get_installed_xbps_packages().unwrap_or_default();
    let _enabled_services = ServiceManager::get_enabled_services().unwrap_or_default();

    if json {
        let output = StatusOutput {
            host: config.host.clone(),
            description: config.description.clone(),
            config_file: paths.config_file.display().to_string(),
            enabled_modules: config.enabled_modules.clone(),
            declared_packages: declared.len(),
            installed_xbps: installed_xbps.len(),
            services_enabled: config.services.enabled.clone(),
            services_disabled: config.services.disabled.clone(),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!("{}", "=== vcli status ===".blue().bold());
    println!();
    println!("{}: {}", "Host".bold(), config.host);
    if !config.description.is_empty() {
        println!("{}: {}", "Description".bold(), config.description);
    }
    println!("{}: {}", "Config".bold(), paths.config_file.display());
    println!();

    // Modules
    println!("{} ({}):", "Enabled Modules".bold(), config.enabled_modules.len());
    if config.enabled_modules.is_empty() {
        println!("  (none)");
    } else {
        for module in &config.enabled_modules {
            let found = pkg_manager.find_module_path(module).is_some();
            if found {
                println!("  {} {}", "✓".green(), module);
            } else {
                println!("  {} {} (not found!)", "✗".red(), module);
            }
        }
    }
    println!();

    // Packages
    println!("{}: {}", "Declared packages".bold(), declared.len());
    println!("{}: {}", "Installed xbps packages (manual)".bold(), installed_xbps.len());
    println!();

    // Services
    println!("{} (declared):", "Services".bold());
    if config.services.enabled.is_empty() && config.services.disabled.is_empty() {
        println!("  (none configured)");
    } else {
        for svc in &config.services.enabled {
            let is_enabled = ServiceManager::is_enabled(svc);
            if is_enabled {
                println!("  {} {} (enabled)", "✓".green(), svc);
            } else {
                println!("  {} {} (declared but not enabled!)", "!".yellow(), svc);
            }
        }
        for svc in &config.services.disabled {
            let is_enabled = ServiceManager::is_enabled(svc);
            if !is_enabled {
                println!("  {} {} (disabled)", "✓".green(), svc);
            } else {
                println!("  {} {} (declared disabled but still running!)", "!".yellow(), svc);
            }
        }
    }
    println!();

    // Settings
    println!("{}", "Settings:".bold());
    println!("  flatpak_scope: {:?}", config.flatpak_scope);
    println!("  auto_prune: {}", config.auto_prune);
    println!("  module_processing: {:?}", config.module_processing);

    Ok(())
}
