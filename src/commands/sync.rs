use anyhow::{anyhow, Context, Result};
use colored::*;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::config::{load_config, Config, ConfigPaths, FlatpakScope, ModuleProcessing, PackageType};
use crate::package::{get_flatpak_version, get_package_version, Package, PackageManager};
use crate::services::{
    create_updated_state, load_services_state, save_services_state, ServiceManager,
};

#[derive(Serialize)]
struct SyncOutput {
    success: bool,
    dry_run: bool,
    summary: SyncSummary,
    actions: SyncActions,
}

#[derive(Serialize)]
struct SyncSummary {
    to_install: usize,
    to_remove: usize,
    flatpak_to_install: usize,
    flatpak_to_remove: usize,
}

#[derive(Serialize)]
struct SyncActions {
    install: Vec<String>,
    remove: Vec<String>,
    flatpak_install: Vec<String>,
    flatpak_remove: Vec<String>,
}

pub fn run(
    paths: &ConfigPaths,
    dry_run: bool,
    prune: bool,
    force: bool,
    no_backup: bool,
    no_hooks: bool,
    force_dotfiles: bool,
    json: bool,
    auto_commit: bool,
) -> Result<()> {
    // Pre-flight validation
    if !json {
        println!("{}", "Running pre-flight validation...".blue());
    }

    let validation_errors = run_preflight_validation(paths, json)?;

    if validation_errors > 0 {
        if !json {
            println!();
            println!("{}", "✗ Validation failed - refusing to sync".red().bold());
            println!();
            println!("Fix the errors above before running 'vcli sync'.");
            println!("You can run 'vcli validate' to see detailed validation results.");
        }
        anyhow::bail!("Validation failed with {} error(s)", validation_errors);
    }

    if !json {
        println!("{}", "✓ Validation passed".green());
        println!();
    }

    let config = load_config(paths)?;
    let should_prune = prune || config.auto_prune;

    // Sequential mode
    if config.module_processing == ModuleProcessing::Sequential {
        if !dry_run {
            sync_modules_sequential(paths, &config, force_dotfiles, json)?;
            sync_services(paths, &config, json)?;
        } else {
            println!();
            println!(
                "{}",
                "Dry run mode not fully supported for sequential processing — showing service diff only".yellow()
            );
            show_service_preview(paths, &config)?;
        }

        update_state_file(paths, &PackageManager::new(paths.clone()).get_declared_packages(&config)?)?;

        if !json {
            println!();
            println!("{}", "Sync complete!".green());
        }
        return Ok(());
    }

    // === PARALLEL MODE ===
    let pkg_manager = PackageManager::new(paths.clone());

    if !json {
        println!("{}", "Loading package configuration...".blue());
    }

    let declared = pkg_manager.get_declared_packages(&config)?;

    if !json {
        println!("  {} Loaded {} declared packages", "→".blue(), declared.len());
    }

    // Get installed packages
    let installed_xbps = pkg_manager.get_installed_xbps_packages()?;
    let flatpak_scope = config.flatpak_scope.as_flag();
    let installed_flatpaks = pkg_manager.get_installed_flatpaks(flatpak_scope)?;

    if !json {
        println!(
            "  {} Found {} installed xbps packages",
            "→".blue(),
            installed_xbps.len()
        );
    }

    let installed_xbps_map: HashMap<String, String> = installed_xbps.into_iter().collect();
    let installed_flatpak_set: HashSet<String> = installed_flatpaks.into_iter().collect();

    // Build sync plan
    let mut to_install: Vec<String> = Vec::new();
    let mut flatpak_to_install: Vec<String> = Vec::new();

    for pkg in &declared {
        match pkg.package_type {
            PackageType::Flatpak => {
                if !installed_flatpak_set.contains(&pkg.name) {
                    flatpak_to_install.push(pkg.name.clone());
                }
            }
            PackageType::Xbps => {
                if !installed_xbps_map.contains_key(&pkg.name) {
                    to_install.push(pkg.name.clone());
                }
            }
        }
    }

    // Find packages to remove (if pruning)
    let mut to_remove: Vec<String> = Vec::new();
    let mut flatpak_to_remove: Vec<String> = Vec::new();

    if should_prune {
        if let Ok(state) = load_state_file(paths) {
            let declared_names: HashSet<String> = declared.iter().map(|p| p.name.clone()).collect();

            for state_pkg in state.packages {
                if !declared_names.contains(&state_pkg.name) {
                    match state_pkg.pkg_type.as_deref() {
                        Some("flatpak") => {
                            if installed_flatpak_set.contains(&state_pkg.name) {
                                flatpak_to_remove.push(state_pkg.name);
                            }
                        }
                        _ => {
                            if installed_xbps_map.contains_key(&state_pkg.name) {
                                to_remove.push(state_pkg.name);
                            }
                        }
                    }
                }
            }
        }
    }

    let no_package_changes = to_install.is_empty()
        && to_remove.is_empty()
        && flatpak_to_install.is_empty()
        && flatpak_to_remove.is_empty();

    // Display / JSON output
    if json {
        let output = SyncOutput {
            success: true,
            dry_run,
            summary: SyncSummary {
                to_install: to_install.len(),
                to_remove: to_remove.len(),
                flatpak_to_install: flatpak_to_install.len(),
                flatpak_to_remove: flatpak_to_remove.len(),
            },
            actions: SyncActions {
                install: to_install.clone(),
                remove: to_remove.clone(),
                flatpak_install: flatpak_to_install.clone(),
                flatpak_remove: flatpak_to_remove.clone(),
            },
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        if dry_run {
            return Ok(());
        }
    } else {
        println!();
        println!("{}", "=== Sync Summary ===".blue());

        if !to_install.is_empty() {
            println!("{}", format!("Packages to install: {}", to_install.len()).green());
            for pkg in &to_install {
                println!("  {} {}", "+".green(), pkg);
            }
        }
        if !flatpak_to_install.is_empty() {
            println!("{}", format!("Flatpaks to install: {}", flatpak_to_install.len()).green());
            for pkg in &flatpak_to_install {
                println!("  {} {}", "+".green(), pkg);
            }
        }
        if !to_remove.is_empty() {
            println!("{}", format!("Packages to remove: {}", to_remove.len()).yellow());
            for pkg in &to_remove {
                println!("  {} {}", "-".yellow(), pkg);
            }
        }
        if !flatpak_to_remove.is_empty() {
            println!("{}", format!("Flatpaks to remove: {}", flatpak_to_remove.len()).yellow());
            for pkg in &flatpak_to_remove {
                println!("  {} {}", "-".yellow(), pkg);
            }
        }

        if no_package_changes {
            println!();
            println!("{}", "Packages are already in sync!".green());
        }

        if dry_run {
            show_service_preview(paths, &config)?;
            println!();
            println!("{}", "Dry run - no changes made".blue());
            return Ok(());
        }
    }

    // Confirm unless --force or --json
    if !no_package_changes && !force && !json {
        println!();
        print!("Apply these changes? [y/N] ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("{}", "Cancelled".yellow());
            return Ok(());
        }
    }

    // Config backup
    if !no_backup && !json && config.config_backups.enabled {
        println!("{}", "Creating configuration backup...".blue());
        match crate::commands::config_backup::save_config(paths, "auto-sync", true) {
            Ok(_) => println!("{}", "✓ Configuration backup created".green()),
            Err(e) => {
                println!("{}", format!("⚠ Warning: Failed to create config backup: {}", e).yellow());
                println!("{}", "  Continuing with sync...".yellow());
            }
        }
    }

    // Pre-install hooks
    if !no_hooks {
        run_pre_install_hooks(paths, &config, json)?;
    }

    // Execute package operations
    if !no_package_changes {
        execute_sync(
            paths,
            &to_install,
            &to_remove,
            &flatpak_to_install,
            &flatpak_to_remove,
            &config.flatpak_scope,
            json,
        )?;
    }


    // Sync services
    sync_services(paths, &config, json)?;

    // Post-install hooks
    if !no_hooks {
        run_post_install_hooks(paths, &config, json)?;
    }

    // Update state file
    update_state_file(paths, &declared)?;

    if !json {
        println!();
        println!("{}", "Sync complete!".green());
    }

    // Auto-commit
    let should_auto_commit = auto_commit || config.auto_commit;
    if should_auto_commit && !dry_run {
        if let Err(e) = auto_commit_changes(paths, json) {
            if !json {
                eprintln!("{} Auto-commit failed: {}", "Warning:".yellow(), e);
            }
        }
    }

    Ok(())
}

fn find_suggestions(name: &str) -> Vec<String> {
    let output = std::process::Command::new("xbps-query")
        .args(["-Rs", name])
        .output();
    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.lines().filter_map(|line| {
                let line = line.trim();
                let rest = if line.starts_with("[*]") || line.starts_with("[-]") {
                    line[3..].trim()
                } else { line };
                let pkgver = rest.split_whitespace().next()?;
                let (pkg_name, _) = crate::package::split_xbps_pkgver(pkgver);
                if pkg_name.is_empty() { None } else { Some(pkg_name) }
            }).take(5).collect()
        }
        _ => Vec::new(),
    }
}

fn execute_sync(
    paths: &ConfigPaths,
    to_install: &[String],
    to_remove: &[String],
    flatpak_to_install: &[String],
    flatpak_to_remove: &[String],
    flatpak_scope: &FlatpakScope,
    json: bool,
) -> Result<()> {
    let pkg_manager = PackageManager::new(paths.clone());

    // Install xbps packages — try batch, fall back to one-by-one on failure
    if !to_install.is_empty() {
        if !json {
            println!();
            println!("{}", format!("Installing {} xbps package(s)...", to_install.len()).blue());
        }
        let batch_result = pkg_manager.install_xbps(&to_install.to_vec(), json);
        if batch_result.is_err() {
            if !json {
                println!("{}", "  Batch failed — trying packages one by one...".yellow());
            }
            let mut failed: Vec<(String, Vec<String>)> = Vec::new();
            let mut succeeded = 0usize;
            for pkg in to_install {
                match pkg_manager.install_xbps(&[pkg.clone()], true) {
                    Ok(_) => { succeeded += 1; if !json { println!("  {} {}", "✓".green(), pkg); } }
                    Err(_) => { failed.push((pkg.clone(), find_suggestions(pkg))); }
                }
            }
            if !failed.is_empty() && !json {
                println!();
                println!("{}", format!("  {} package(s) not found:", failed.len()).yellow());
                for (pkg, suggestions) in &failed {
                    println!("  {} {} — not in repos", "✗".red(), pkg);
                    if !suggestions.is_empty() {
                        println!("    {} Did you mean: {}", "?".yellow(), suggestions.join(", ").cyan());
                    } else {
                        println!("    {} Try: {}", "→".blue(), format!("xbps-query -Rs {}", pkg).cyan());
                    }
                }
                println!("  Fix the names in your config, then run vcli sync again.");
            }
            if succeeded > 0 && !json {
                println!("{}", format!("  ✓ {}/{} package(s) installed", succeeded, to_install.len()).green());
            }
        } else if !json {
            println!("{}", "✓ Packages installed successfully".green());
        }
    }

    // Install flatpaks
    if !flatpak_to_install.is_empty() {
        if !json {
            println!();
            println!("{}", format!("Installing {} flatpak(s)...", flatpak_to_install.len()).blue());
        }
        for pkg in flatpak_to_install {
            pkg_manager.install_flatpak(pkg, flatpak_scope)?;
        }
        if !json {
            println!("{}", "✓ Flatpaks installed successfully".green());
        }
    }

    // Remove xbps packages
    if !to_remove.is_empty() {
        if !json {
            println!();
            println!("{}", "Removing packages...".blue());
        }
        pkg_manager.remove_xbps(&to_remove.to_vec(), json)?;
        if !json {
            println!("{}", "✓ Packages removed successfully".green());
        }
    }

    // Remove flatpaks
    if !flatpak_to_remove.is_empty() {
        if !json {
            println!();
            println!("{}", "Removing flatpaks...".blue());
        }
        for pkg in flatpak_to_remove {
            pkg_manager.remove_flatpak(pkg, flatpak_scope)?;
        }
        if !json {
            println!("{}", "✓ Flatpaks removed successfully".green());
        }
    }

    Ok(())
}

fn sync_modules_sequential(
    paths: &ConfigPaths,
    config: &Config,
    _force_dotfiles: bool,
    json: bool,
) -> Result<()> {
    let pkg_manager = PackageManager::new(paths.clone());
    let total_modules = config.enabled_modules.len();

    for (idx, module_name) in config.enabled_modules.iter().enumerate() {
        let module_num = idx + 1;
        println!();
        println!(
            "{} Processing module {}/{}: {}",
            "→".blue(),
            module_num,
            total_modules,
            module_name.cyan()
        );

        // Load module
        let module_path = match pkg_manager.find_module_path(module_name) {
            Some(p) => p,
            None => {
                eprintln!("{} Module '{}' not found — skipping", "!".yellow(), module_name);
                continue;
            }
        };

        let module = match crate::config::load_module(&module_path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("{} Failed to load module '{}': {}", "✗".red(), module_name, e);
                return Err(anyhow!("Module loading failed: {}", module_name));
            }
        };

        // Pre-install hook
        let mut ran_pre_hook = false;
        if let Some(hook_script) = module.pre_install_hook() {
            let hook_path = resolve_hook_path(paths, &module, hook_script);
            if hook_path.exists() && !hook_path.is_dir() {
                let behavior = module.pre_hook_behavior();
                let should_run =
                    match check_hook_status(paths, &format!("pre_{}", module_name), &hook_path)? {
                        HookStatus::Executed => behavior == "always",
                        HookStatus::Skipped => behavior == "always",
                        HookStatus::Modified | HookStatus::NotRun => behavior != "skip",
                    };

                if should_run {
                    if !json {
                        println!("  {} Running pre-install hook...", "→".blue());
                    }
                    execute_hook(&module, hook_script, module_name, paths, json, module.run_hooks_as_user())?;
                    mark_hook_executed(paths, &format!("pre_{}", module_name), &hook_path)?;
                    ran_pre_hook = true;
                }
            }
        }

        // Refresh xbps repos after hook (might have added a repo)
        if ran_pre_hook {
            println!("  {} Syncing xbps repos...", "→".blue());
            let _ = std::process::Command::new("xbps-install")
                .args(["-S"])
                .status();
        }

        // Install module packages
        let module_packages: Vec<Package> = module.packages().iter().map(Package::from).collect();

        if !module_packages.is_empty() {
            let installed_xbps = pkg_manager.get_installed_xbps_packages()?;
            let installed_xbps_map: HashMap<String, String> = installed_xbps.into_iter().collect();
            let installed_flatpaks = pkg_manager.get_installed_flatpaks(config.flatpak_scope.as_flag())?;
            let installed_flatpak_set: HashSet<String> = installed_flatpaks.into_iter().collect();

            let mut xbps_to_install: Vec<String> = Vec::new();
            let mut flatpak_to_install: Vec<String> = Vec::new();

            for pkg in &module_packages {
                match pkg.package_type {
                    PackageType::Xbps => {
                        if !installed_xbps_map.contains_key(&pkg.name) {
                            xbps_to_install.push(pkg.name.clone());
                        }
                    }
                    PackageType::Flatpak => {
                        if !installed_flatpak_set.contains(&pkg.name) {
                            flatpak_to_install.push(pkg.name.clone());
                        }
                    }
                }
            }

            if xbps_to_install.is_empty() && flatpak_to_install.is_empty() {
                println!(
                    "  {} All {} package(s) already installed",
                    "✓".green(),
                    module_packages.len()
                );
            } else {
                if !xbps_to_install.is_empty() {
                    println!(
                        "  {} Installing {} xbps package(s)...",
                        "→".blue(),
                        xbps_to_install.len()
                    );
                    pkg_manager.install_xbps(&xbps_to_install, true)?;
                }
                for app_id in &flatpak_to_install {
                    println!("  {} Installing flatpak: {}", "→".blue(), app_id);
                    pkg_manager.install_flatpak(app_id, &config.flatpak_scope)?;
                }
                println!("  {} Module packages installed", "✓".green());
            }
        }

        // Post-install hook
        if let Some(hook_script) = module.post_install_hook() {
            let hook_path = resolve_hook_path(paths, &module, hook_script);
            if hook_path.exists() && !hook_path.is_dir() {
                let behavior = module.post_hook_behavior();
                let should_run =
                    match check_hook_status(paths, module_name, &hook_path)? {
                        HookStatus::Executed => behavior == "always",
                        HookStatus::Skipped => behavior == "always",
                        HookStatus::Modified | HookStatus::NotRun => behavior != "skip",
                    };

                if should_run {
                    if !json {
                        println!("  {} Running post-install hook...", "→".blue());
                    }
                    execute_hook(&module, hook_script, module_name, paths, json, module.run_hooks_as_user())?;
                    mark_hook_executed(paths, module_name, &hook_path)?;
                }
            }
        }

        println!("  {} Module {} complete", "✓".green(), module_name.cyan());
    }

    Ok(())
}

fn sync_services(paths: &ConfigPaths, config: &Config, json: bool) -> Result<()> {
    if config.services.enabled.is_empty() && config.services.disabled.is_empty() {
        return Ok(());
    }

    if !json {
        println!();
        println!("{}", "Syncing services...".blue());
    }

    let previous_state = load_services_state(&paths.services_state_file)
        .context("Failed to load previous services state")?;

    let report = ServiceManager::sync_services(
        &config.services.enabled,
        &config.services.disabled,
        &previous_state,
    )
    .context("Failed to sync services")?;

    let new_state = create_updated_state(&config.services.enabled, &config.services.disabled);
    save_services_state(&paths.services_state_file, &new_state)
        .context("Failed to save services state")?;

    if report.has_errors() && !json {
        eprintln!("{}: Some service operations failed.", "Warning".yellow());
    }

    if !json && !report.enabled.is_empty() {
        println!("{} Services synced", "✓".green());
    }

    Ok(())
}

fn show_service_preview(paths: &ConfigPaths, config: &Config) -> Result<()> {
    let previous_state = load_services_state(&paths.services_state_file)?;
    let preview = ServiceManager::preview_services(
        &config.services.enabled,
        &config.services.disabled,
        &previous_state,
    )?;

    if preview.has_changes() {
        println!();
        println!("{}", "=== Service Changes ===".blue());
        for svc in &preview.services_to_enable {
            println!("  {} {}", "+".green(), svc);
        }
        for svc in &preview.services_to_disable {
            println!("  {} {}", "-".yellow(), svc);
        }
    } else {
        println!();
        println!("{}", "Services are already in sync!".green());
    }
    Ok(())
}

// ===== Hook management =====

fn resolve_hook_path(
    paths: &ConfigPaths,
    module: &crate::config::ModuleStructure,
    hook_script: &str,
) -> PathBuf {
    if Path::new(hook_script).is_absolute() {
        PathBuf::from(hook_script)
    } else if module.is_directory() {
        module.root_dir().join(hook_script)
    } else {
        paths.config_dir.join(hook_script)
    }
}

fn execute_hook(
    module: &crate::config::ModuleStructure,
    hook_script: &str,
    module_name: &str,
    paths: &ConfigPaths,
    json: bool,
    run_as_user: Option<String>,
) -> Result<()> {
    let hook_path = resolve_hook_path(paths, module, hook_script);

    if !hook_path.exists() {
        return Err(anyhow!("Hook script not found: {:?}", hook_path));
    }

    let status = match run_as_user {
        Some(username) => std::process::Command::new("sudo")
            .args(["-u", &username, "bash", hook_path.to_str().unwrap()])
            .stdin(std::process::Stdio::inherit())
            .stdout(if json { std::process::Stdio::null() } else { std::process::Stdio::inherit() })
            .stderr(if json { std::process::Stdio::null() } else { std::process::Stdio::inherit() })
            .status()
            .context(format!("Failed to execute hook for {}", module_name))?,
        None => std::process::Command::new("bash")
            .args([hook_path.to_str().unwrap()])
            .stdin(std::process::Stdio::inherit())
            .stdout(if json { std::process::Stdio::null() } else { std::process::Stdio::inherit() })
            .stderr(if json { std::process::Stdio::null() } else { std::process::Stdio::inherit() })
            .status()
            .context(format!("Failed to execute hook for {}", module_name))?,
    };

    if !status.success() {
        return Err(anyhow!(
            "Hook failed for module {} (exit code: {})",
            module_name,
            status.code().unwrap_or(-1)
        ));
    }
    Ok(())
}

fn run_pre_install_hooks(paths: &ConfigPaths, config: &Config, json: bool) -> Result<()> {
    run_hooks(paths, config, json, true)
}

fn run_post_install_hooks(paths: &ConfigPaths, config: &Config, json: bool) -> Result<()> {
    run_hooks(paths, config, json, false)
}

fn run_hooks(paths: &ConfigPaths, config: &Config, json: bool, pre: bool) -> Result<()> {
    let pkg_manager = PackageManager::new(paths.clone());

    for module_name in &config.enabled_modules {
        let module_path = match pkg_manager.find_module_path(module_name) {
            Some(p) => p,
            None => continue,
        };

        let module = match crate::config::load_module(&module_path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let (hook_script, behavior, state_key) = if pre {
            (
                module.pre_install_hook(),
                module.pre_hook_behavior().to_string(),
                format!("pre_{}", module_name),
            )
        } else {
            (
                module.post_install_hook(),
                module.post_hook_behavior().to_string(),
                module_name.clone(),
            )
        };

        let hook_script = match hook_script {
            Some(s) => s,
            None => continue,
        };

        let hook_path = resolve_hook_path(paths, &module, hook_script);
        if !hook_path.exists() || hook_path.is_dir() {
            continue;
        }

        let should_run = match check_hook_status(paths, &state_key, &hook_path)? {
            HookStatus::Executed => behavior == "always",
            HookStatus::Skipped => behavior == "always",
            HookStatus::Modified | HookStatus::NotRun => behavior != "skip",
        };

        if !should_run {
            continue;
        }

        let run = if behavior == "always" || behavior == "once" {
            if !json {
                let hook_type = if pre { "pre" } else { "post" };
                println!();
                println!(
                    "{}",
                    format!("{}-install hook for module '{}' ({})", hook_type, module_name, behavior)
                        .blue()
                        .bold()
                );
            }
            true
        } else if !json {
            // Ask user
            println!();
            let hook_type = if pre { "pre" } else { "post" };
            println!(
                "{}",
                format!("{}-install hook for module '{}'", hook_type, module_name)
                    .blue()
                    .bold()
            );
            println!("  Script: {}", hook_path.display().to_string().dimmed());
            println!();
            print!("Run this hook? [Y/n/s] (Y=yes, n=no this time, s=skip permanently): ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let choice = input.trim().to_lowercase();
            match choice.as_str() {
                "n" => {
                    println!("{}", "Skipping this time".yellow());
                    false
                }
                "s" => {
                    println!("{}", "Marking as permanently skipped".yellow());
                    mark_hook_skipped(paths, &state_key)?;
                    false
                }
                _ => true,
            }
        } else {
            true
        };

        if !run {
            continue;
        }

        execute_hook(&module, hook_script, module_name, paths, json, module.run_hooks_as_user())?;
        mark_hook_executed(paths, &state_key, &hook_path)?;

        if !json {
            println!("{}", "✓ Hook completed successfully".green());
        }
    }

    Ok(())
}

// ===== Pre-flight validation =====

fn run_preflight_validation(paths: &ConfigPaths, json: bool) -> Result<usize> {
    let mut total_errors = 0;
    let mut total_warnings = 0;

    let config = match load_config(paths) {
        Ok(c) => c,
        Err(e) => {
            if !json {
                println!("  {} Failed to load config: {}", "✗".red(), e);
            }
            return Ok(1);
        }
    };

    let pkg_manager = PackageManager::new(paths.clone());

    for module_name in &config.enabled_modules {
        let module_path = match pkg_manager.find_module_path(module_name) {
            Some(p) => p,
            None => {
                if !json {
                    println!("  {} Module not found: {}", "✗".red(), module_name);
                }
                total_errors += 1;
                continue;
            }
        };

        match crate::config::load_module(&module_path) {
            Ok(module) => {
                let validation = crate::config::validate_module(&module, module_name);
                if !validation.is_clean() {
                    if !json {
                        let module_type = if module.is_directory() { "directory" } else { "yaml" };
                        println!("  {} Module '{}' ({}):", "→".blue(), module_name, module_type);
                        for error in &validation.errors {
                            println!("    {} {}", "✗".red(), error);
                        }
                        for warning in &validation.warnings {
                            println!("    {} {}", "⚠".yellow(), warning);
                        }
                    }
                    total_errors += validation.errors.len();
                    total_warnings += validation.warnings.len();
                }
            }
            Err(e) => {
                if !json {
                    println!("  {} Failed to load module '{}': {}", "✗".red(), module_name, e);
                }
                total_errors += 1;
            }
        }
    }

    if !json && total_warnings > 0 && total_errors == 0 {
        println!();
        println!("  {} {} warning(s) found", "⚠".yellow(), total_warnings);
    }

    Ok(total_errors)
}

// ===== State file management =====

#[derive(Debug)]
#[allow(dead_code)]
struct StatePackage {
    name: String,
    version: Option<String>,
    pkg_type: Option<String>,
}

struct StateFile {
    packages: Vec<StatePackage>,
}

fn load_state_file(paths: &ConfigPaths) -> Result<StateFile> {
    use serde_yaml::Value;

    let content = std::fs::read_to_string(&paths.state_file).context("Failed to read state file")?;
    let yaml: Value = serde_yaml::from_str(&content).context("Failed to parse state file")?;

    let mut packages = Vec::new();
    if let Some(pkgs) = yaml.get("packages").and_then(|v| v.as_sequence()) {
        for pkg in pkgs {
            if let Some(pkg_map) = pkg.as_mapping() {
                let name = pkg_map
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let version = pkg_map
                    .get("version")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let pkg_type = pkg_map
                    .get("type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                if !name.is_empty() {
                    packages.push(StatePackage { name, version, pkg_type });
                }
            }
        }
    }
    Ok(StateFile { packages })
}

fn update_state_file(paths: &ConfigPaths, declared: &[Package]) -> Result<()> {
    use chrono::Utc;
    use std::io::Write as _;

    std::fs::create_dir_all(&paths.state_dir).context("Failed to create state directory")?;

    let mut file = std::fs::File::create(&paths.state_file).context("Failed to create state file")?;

    writeln!(file, "# Auto-generated state file - tracks vcli-managed packages")?;
    writeln!(file, "# Generated: {}", Utc::now().to_rfc3339())?;
    writeln!(file)?;
    writeln!(file, "packages:")?;

    for pkg in declared {
        writeln!(file, "  - name: {}", pkg.name)?;

        let version = if pkg.package_type == PackageType::Flatpak {
            get_flatpak_version(&pkg.name)
        } else {
            get_package_version(&pkg.name)
        };

        if let Some(ver) = version {
            writeln!(file, "    version: \"{}\"", ver)?;
        }

        let type_str = match pkg.package_type {
            PackageType::Flatpak => "flatpak",
            PackageType::Xbps => "xbps",
        };
        writeln!(file, "    type: {}", type_str)?;
    }

    Ok(())
}

// ===== Hook state helpers =====

pub enum HookStatus {
    NotRun,
    Executed,
    Skipped,
    Modified,
}

pub fn check_hook_status(
    paths: &ConfigPaths,
    module: &str,
    hook_path: &PathBuf,
) -> Result<HookStatus> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    if !paths.hooks_state_file.exists() {
        return Ok(HookStatus::NotRun);
    }

    let content = std::fs::read_to_string(&paths.hooks_state_file)?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(&content)?;

    let script_content = std::fs::read_to_string(hook_path)?;
    let mut hasher = DefaultHasher::new();
    script_content.hash(&mut hasher);
    let current_hash = hasher.finish().to_string();

    if let Some(hooks) = yaml.get("hooks").and_then(|v| v.as_sequence()) {
        for hook in hooks {
            if let Some(hook_map) = hook.as_mapping() {
                let stored_module = hook_map
                    .get("module")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if stored_module == module {
                    let status = hook_map
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("executed");

                    if status == "skipped" {
                        return Ok(HookStatus::Skipped);
                    }

                    let stored_hash = hook_map
                        .get("script_hash")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    if stored_hash == current_hash {
                        return Ok(HookStatus::Executed);
                    } else {
                        return Ok(HookStatus::Modified);
                    }
                }
            }
        }
    }

    Ok(HookStatus::NotRun)
}

pub fn mark_hook_skipped(paths: &ConfigPaths, module: &str) -> Result<()> {
    let mut yaml: serde_yaml::Value = if paths.hooks_state_file.exists() {
        let content = std::fs::read_to_string(&paths.hooks_state_file)?;
        serde_yaml::from_str(&content)?
    } else {
        serde_yaml::from_str("hooks: []")?
    };

    if let Some(hooks) = yaml.get_mut("hooks").and_then(|v| v.as_sequence_mut()) {
        hooks.retain(|hook| {
            hook.get("module")
                .and_then(|v| v.as_str())
                .map(|m| m != module)
                .unwrap_or(true)
        });
        let new_hook = serde_yaml::to_value(serde_json::json!({
            "module": module,
            "status": "skipped",
        }))?;
        hooks.push(new_hook);
    }

    std::fs::create_dir_all(paths.state_dir.as_path())?;
    let content = serde_yaml::to_string(&yaml)?;
    std::fs::write(&paths.hooks_state_file, content)?;
    Ok(())
}

pub fn mark_hook_executed(
    paths: &ConfigPaths,
    module: &str,
    hook_path: &PathBuf,
) -> Result<()> {
    use chrono::Utc;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let script_content = std::fs::read_to_string(hook_path)?;
    let mut hasher = DefaultHasher::new();
    script_content.hash(&mut hasher);
    let script_hash = hasher.finish().to_string();
    let timestamp = Utc::now().to_rfc3339();

    let mut yaml: serde_yaml::Value = if paths.hooks_state_file.exists() {
        let content = std::fs::read_to_string(&paths.hooks_state_file)?;
        serde_yaml::from_str(&content)?
    } else {
        serde_yaml::from_str("hooks: []")?
    };

    if let Some(hooks) = yaml.get_mut("hooks").and_then(|v| v.as_sequence_mut()) {
        hooks.retain(|hook| {
            hook.get("module")
                .and_then(|v| v.as_str())
                .map(|m| m != module)
                .unwrap_or(true)
        });
        let new_hook = serde_yaml::to_value(serde_json::json!({
            "module": module,
            "script": hook_path.to_string_lossy().to_string(),
            "script_hash": script_hash,
            "executed_at": timestamp,
            "status": "executed",
        }))?;
        hooks.push(new_hook);
    }

    std::fs::create_dir_all(paths.state_dir.as_path())?;
    let content = serde_yaml::to_string(&yaml)?;
    std::fs::write(&paths.hooks_state_file, content)?;
    Ok(())
}

fn auto_commit_changes(paths: &ConfigPaths, json: bool) -> Result<()> {
    use std::process::Command;

    let git_dir = paths.config_dir.join(".git");
    if !git_dir.exists() {
        anyhow::bail!("Not a git repository");
    }

    let status_output = Command::new("git")
        .args(["-C", paths.config_dir.to_str().unwrap(), "status", "--porcelain"])
        .output()?;

    if status_output.stdout.is_empty() {
        return Ok(());
    }

    if !json {
        println!("{}", "Auto-committing changes to git...".blue());
    }

    Command::new("git")
        .args(["-C", paths.config_dir.to_str().unwrap(), "add", "."])
        .status()?;

    let hostname = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string());

    let commit_message = format!("Synced changes from {}", hostname);
    Command::new("git")
        .args(["-C", paths.config_dir.to_str().unwrap(), "commit", "-m", &commit_message])
        .status()?;

    if !json {
        println!("{}", "Changes committed successfully".green());
    }

    Ok(())
}
