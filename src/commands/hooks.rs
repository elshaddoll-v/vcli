use anyhow::Result;
use colored::*;

use crate::config::{load_config, ConfigPaths};
use crate::package::PackageManager;

pub fn list(paths: &ConfigPaths, json: bool) -> Result<()> {
    let config = load_config(paths)?;
    let pkg_manager = PackageManager::new(paths.clone());

    if !json {
        println!("{}", "Post-install hooks:".blue().bold());
        println!();
    }

    let mut any = false;
    for module_name in &config.enabled_modules {
        if let Some(module_path) = pkg_manager.find_module_path(module_name) {
            if let Ok(module) = crate::config::load_module(&module_path) {
                let has_pre = module.pre_install_hook().is_some();
                let has_post = module.post_install_hook().is_some();

                if !has_pre && !has_post {
                    continue;
                }

                any = true;

                if !json {
                    println!("  {} {}:", "→".blue(), module_name.bold());

                    if let Some(hook) = module.pre_install_hook() {
                        let hook_path = if module.is_directory() {
                            module.root_dir().join(hook)
                        } else {
                            paths.config_dir.join(hook)
                        };

                        let status_str = match crate::commands::sync::check_hook_status(
                            paths,
                            &format!("pre_{}", module_name),
                            &hook_path,
                        ) {
                            Ok(crate::commands::sync::HookStatus::Executed) => {
                                "[executed]".green().to_string()
                            }
                            Ok(crate::commands::sync::HookStatus::Skipped) => {
                                "[skipped]".yellow().to_string()
                            }
                            Ok(crate::commands::sync::HookStatus::Modified) => {
                                "[modified]".cyan().to_string()
                            }
                            _ => "[not run]".dimmed().to_string(),
                        };
                        println!(
                            "    pre:  {} {} (behavior: {})",
                            hook,
                            status_str,
                            module.pre_hook_behavior()
                        );
                    }

                    if let Some(hook) = module.post_install_hook() {
                        let hook_path = if module.is_directory() {
                            module.root_dir().join(hook)
                        } else {
                            paths.config_dir.join(hook)
                        };

                        let status_str = match crate::commands::sync::check_hook_status(
                            paths,
                            module_name,
                            &hook_path,
                        ) {
                            Ok(crate::commands::sync::HookStatus::Executed) => {
                                "[executed]".green().to_string()
                            }
                            Ok(crate::commands::sync::HookStatus::Skipped) => {
                                "[skipped]".yellow().to_string()
                            }
                            Ok(crate::commands::sync::HookStatus::Modified) => {
                                "[modified]".cyan().to_string()
                            }
                            _ => "[not run]".dimmed().to_string(),
                        };
                        println!(
                            "    post: {} {} (behavior: {})",
                            hook,
                            status_str,
                            module.post_hook_behavior()
                        );
                    }
                    println!();
                }
            }
        }
    }

    if !any && !json {
        println!("  (no hooks configured in enabled modules)");
    }

    Ok(())
}

pub fn reset(paths: &ConfigPaths, module: &str, pre: bool) -> Result<()> {
    let key = if pre {
        format!("pre_{}", module)
    } else {
        module.to_string()
    };

    if !paths.hooks_state_file.exists() {
        println!("{}", "No hooks state file found.".yellow());
        return Ok(());
    }

    let content = std::fs::read_to_string(&paths.hooks_state_file)?;
    let mut yaml: serde_yaml::Value = serde_yaml::from_str(&content)?;

    if let Some(hooks) = yaml.get_mut("hooks").and_then(|v| v.as_sequence_mut()) {
        let before = hooks.len();
        hooks.retain(|hook| {
            hook.get("module")
                .and_then(|v| v.as_str())
                .map(|m| m != key)
                .unwrap_or(true)
        });
        if hooks.len() < before {
            let content = serde_yaml::to_string(&yaml)?;
            std::fs::write(&paths.hooks_state_file, content)?;
            println!(
                "{} Hook for '{}' reset — will run on next sync",
                "✓".green(),
                key
            );
        } else {
            println!("{} No hook state found for '{}'", "!".yellow(), key);
        }
    }

    Ok(())
}

pub fn skip(paths: &ConfigPaths, module: &str, pre: bool) -> Result<()> {
    let key = if pre {
        format!("pre_{}", module)
    } else {
        module.to_string()
    };

    crate::commands::sync::mark_hook_skipped(paths, &key)?;
    println!("{} Hook for '{}' marked as skipped", "✓".green(), key);
    Ok(())
}

pub fn run_hook(paths: &ConfigPaths, module: &str, pre: bool) -> Result<()> {
    let pkg_manager = PackageManager::new(paths.clone());
    let module_path = pkg_manager
        .find_module_path(module)
        .ok_or_else(|| anyhow::anyhow!("Module '{}' not found", module))?;

    let m = crate::config::load_module(&module_path)?;

    let hook_script = if pre {
        m.pre_install_hook()
    } else {
        m.post_install_hook()
    };

    let hook_script = hook_script
        .ok_or_else(|| anyhow::anyhow!("Module '{}' has no {} hook", module, if pre { "pre-install" } else { "post-install" }))?;

    let hook_path = if m.is_directory() {
        m.root_dir().join(hook_script)
    } else {
        paths.config_dir.join(hook_script)
    };

    if !hook_path.exists() {
        anyhow::bail!("Hook script not found: {:?}", hook_path);
    }

    println!(
        "{} Running {} hook for '{}'...",
        "→".blue(),
        if pre { "pre-install" } else { "post-install" },
        module
    );

    let status = std::process::Command::new("bash")
        .arg(&hook_path)
        .status()?;

    if status.success() {
        let key = if pre { format!("pre_{}", module) } else { module.to_string() };
        crate::commands::sync::mark_hook_executed(paths, &key, &hook_path)?;
        println!("{} Hook completed successfully", "✓".green());
    } else {
        anyhow::bail!("Hook failed with exit code: {}", status.code().unwrap_or(-1));
    }

    Ok(())
}
