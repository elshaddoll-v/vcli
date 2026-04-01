use anyhow::Result;
use colored::*;
use std::collections::HashSet;

use crate::config::{load_config, ConfigPaths, PackageEntry, PackageList};
use crate::package::PackageManager;
use crate::services::ServiceManager;

pub fn run(paths: &ConfigPaths, dry_run: bool, services: bool) -> Result<()> {
    if services {
        return merge_services(paths, dry_run);
    }
    merge_packages(paths, dry_run)
}

fn merge_packages(paths: &ConfigPaths, dry_run: bool) -> Result<()> {
    let config = load_config(paths)?;
    let pkg_manager = PackageManager::new(paths.clone());

    // Get all declared packages
    let declared = pkg_manager.get_declared_packages(&config)?;
    let declared_names: HashSet<String> = declared.iter().map(|p| p.name.clone()).collect();

    // Get all manually installed xbps packages
    let installed = pkg_manager.get_installed_xbps_packages()?;

    // Find packages that are installed but not declared
    let mut to_add: Vec<String> = Vec::new();
    for (name, _version) in &installed {
        if !declared_names.contains(name) {
            to_add.push(name.clone());
        }
    }

    to_add.sort();

    if to_add.is_empty() {
        println!("{}", "✓ All installed packages are already tracked in config.".green());
        return Ok(());
    }

    println!("{}", format!("Found {} untracked packages:", to_add.len()).blue());
    for pkg in &to_add {
        println!("  {} {}", "+".green(), pkg);
    }

    if dry_run {
        println!();
        println!("{}", "Dry run - no changes made.".blue());
        return Ok(());
    }

    // Add to system-packages-{host}/packages.yaml
    let sys_pkg_dir = paths.modules_dir().join(format!("system-packages-{}", config.host));
    std::fs::create_dir_all(&sys_pkg_dir)?;

    let sys_pkg_file = sys_pkg_dir.join("packages.yaml");

    let mut list: PackageList = if sys_pkg_file.exists() {
        let content = std::fs::read_to_string(&sys_pkg_file)?;
        serde_yaml::from_str(&content).unwrap_or_default()
    } else {
        PackageList {
            description: format!("System packages for {}", config.host),
            packages: Vec::new(),
            exclude: Vec::new(),
            conflicts: Vec::new(),
            pre_install_hook: None,
            post_install_hook: None,
            hook_behavior: "ask".to_string(),
            pre_hook_behavior: None,
            post_hook_behavior: None,
            run_hooks_as_user: crate::config::RunHooksAsUser::Bool(false),
            post_disable_hook: None,
            post_disable_behavior: None,
        }
    };

    let existing_names: HashSet<String> = list.packages.iter().map(|p| p.name().to_string()).collect();

    let mut added = 0;
    for pkg in &to_add {
        if !existing_names.contains(pkg) {
            list.packages.push(PackageEntry::Simple(pkg.clone()));
            added += 1;
        }
    }

    let yaml = serde_yaml::to_string(&list)?;
    std::fs::write(&sys_pkg_file, yaml)?;

    println!();
    println!("{}", format!("✓ Added {} packages to {:?}", added, sys_pkg_file).green());

    Ok(())
}

fn merge_services(paths: &ConfigPaths, dry_run: bool) -> Result<()> {
    let enabled_services = ServiceManager::get_enabled_services()?;

    let config = load_config(paths)?;
    let already_declared: HashSet<String> = config.services.enabled.iter().cloned().collect();

    let to_add: Vec<String> = enabled_services
        .into_iter()
        .filter(|s| !already_declared.contains(s))
        .collect();

    if to_add.is_empty() {
        println!("{}", "✓ All enabled services are already in config.".green());
        return Ok(());
    }

    println!("{}", format!("Found {} untracked services:", to_add.len()).blue());
    for svc in &to_add {
        println!("  {} {}", "+".green(), svc);
    }

    if dry_run {
        println!();
        println!("{}", "Dry run - no changes made.".blue());
        return Ok(());
    }

    // Add to host config
    let config_path = crate::config::resolve_config_path(paths)?;
    let content = std::fs::read_to_string(&config_path)?;
    let mut config: crate::config::Config = serde_yaml::from_str(&content)?;

    for svc in &to_add {
        if !config.services.enabled.contains(svc) {
            config.services.enabled.push(svc.clone());
        }
    }

    let yaml = serde_yaml::to_string(&config)?;
    std::fs::write(&config_path, yaml)?;

    println!();
    println!("{}", format!("✓ Added {} services to config", to_add.len()).green());

    Ok(())
}


