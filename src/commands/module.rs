use anyhow::{Context, Result};
use colored::*;
use serde::Serialize;
use std::io::{self, Write};

use crate::config::{load_config, ConfigPaths};
use crate::package::PackageManager;

#[derive(Serialize)]
struct ModuleInfo {
    name: String,
    enabled: bool,
    description: String,
    packages: usize,
}

pub fn list(paths: &ConfigPaths, json: bool) -> Result<()> {
    let config = load_config(paths)?;
    let modules_dir = paths.modules_dir();

    if !modules_dir.exists() {
        if json {
            println!("[]");
        } else {
            println!("{}", "No modules directory found.".yellow());
            println!("Run 'vcli init' first.");
        }
        return Ok(());
    }

    let mut module_infos: Vec<ModuleInfo> = Vec::new();

    // Discover all modules
    for entry in std::fs::read_dir(&modules_dir)
        .context("Failed to read modules directory")?
    {
        let entry = entry?;
        let path = entry.path();

        let name = if path.is_file()
            && path.extension().map(|e| e == "yaml").unwrap_or(false)
        {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            // Skip special files
            if matches!(stem, "base" | "declared-packages") || stem.starts_with("system-packages") {
                continue;
            }
            stem.to_string()
        } else if path.is_dir() {
            if path.file_name().and_then(|n| n.to_str()).map(|n| n.starts_with('.')).unwrap_or(false) {
                continue;
            }
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string()
        } else {
            continue;
        };

        if name.is_empty() {
            continue;
        }

        let enabled = config.enabled_modules.contains(&name);

        let (description, pkg_count) = match crate::config::load_module(&path) {
            Ok(module) => (module.description().to_string(), module.packages().len()),
            Err(_) => ("(failed to load)".to_string(), 0),
        };

        module_infos.push(ModuleInfo {
            name,
            enabled,
            description,
            packages: pkg_count,
        });
    }

    module_infos.sort_by(|a, b| a.name.cmp(&b.name));

    if json {
        println!("{}", serde_json::to_string_pretty(&module_infos)?);
        return Ok(());
    }

    if module_infos.is_empty() {
        println!("{}", "No modules found.".yellow());
        return Ok(());
    }

    println!("{}", "Available modules:".blue().bold());
    println!();
    for m in &module_infos {
        let status = if m.enabled {
            format!("[{}]", "enabled".green())
        } else {
            format!("[{}]", "disabled".dimmed())
        };
        println!(
            "  {} {} - {} ({} packages)",
            status,
            m.name.bold(),
            m.description,
            m.packages
        );
    }

    println!();
    let enabled_count = module_infos.iter().filter(|m| m.enabled).count();
    println!(
        "{}/{} modules enabled",
        enabled_count.to_string().green(),
        module_infos.len()
    );

    Ok(())
}

pub fn enable(paths: &ConfigPaths, name: &str, json: bool) -> Result<()> {
    let config_path = crate::config::resolve_config_path(paths)?;
    let content = std::fs::read_to_string(&config_path)?;
    let mut config: crate::config::Config = serde_yaml::from_str(&content)?;

    if config.enabled_modules.contains(&name.to_string()) {
        if !json {
            println!("{}", format!("Module '{}' is already enabled.", name).yellow());
        }
        return Ok(());
    }

    // Check that module exists
    let pkg_manager = PackageManager::new(paths.clone());
    if pkg_manager.find_module_path(name).is_none() {
        anyhow::bail!("Module '{}' not found in modules directory", name);
    }

    // Check conflicts
    let module_path = pkg_manager.find_module_path(name).unwrap();
    let module = crate::config::load_module(&module_path)?;
    for conflict in module.conflicts() {
        if config.enabled_modules.contains(conflict) {
            anyhow::bail!(
                "Module '{}' conflicts with already-enabled module '{}'",
                name,
                conflict
            );
        }
    }

    config.enabled_modules.push(name.to_string());

    let yaml = serde_yaml::to_string(&config)?;
    std::fs::write(&config_path, yaml)?;

    if !json {
        println!("{}", format!("✓ Module '{}' enabled.", name).green());
        println!("Run 'vcli sync' to install its packages.");
    }

    Ok(())
}

pub fn disable(paths: &ConfigPaths, name: &str, json: bool) -> Result<()> {
    let config_path = crate::config::resolve_config_path(paths)?;
    let content = std::fs::read_to_string(&config_path)?;
    let mut config: crate::config::Config = serde_yaml::from_str(&content)?;

    if !config.enabled_modules.contains(&name.to_string()) {
        if !json {
            println!("{}", format!("Module '{}' is not enabled.", name).yellow());
        }
        return Ok(());
    }

    config.enabled_modules.retain(|m| m != name);

    let yaml = serde_yaml::to_string(&config)?;
    std::fs::write(&config_path, yaml)?;

    if !json {
        println!("{}", format!("✓ Module '{}' disabled.", name).green());
        println!("Run 'vcli sync --prune' to remove its packages.");
    }

    Ok(())
}

pub fn enable_interactive(paths: &ConfigPaths) -> Result<()> {
    // Use fzf if available, else prompt
    let modules = list_module_names(paths)?;
    let config = load_config(paths)?;

    let available: Vec<String> = modules
        .into_iter()
        .filter(|m| !config.enabled_modules.contains(m))
        .collect();

    if available.is_empty() {
        println!("{}", "All modules are already enabled.".yellow());
        return Ok(());
    }

    if which::which("fzf").is_ok() {
        let input = available.join("\n");
        let output = std::process::Command::new("fzf")
            .args(["--multi", "--prompt=Enable module> "])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(stdin) = child.stdin.take() {
                    let mut stdin = stdin;
                    let _ = stdin.write_all(input.as_bytes());
                }
                child.wait_with_output()
            });

        if let Ok(out) = output {
            let selected = String::from_utf8_lossy(&out.stdout);
            for name in selected.lines() {
                let name = name.trim();
                if !name.is_empty() {
                    enable(paths, name, false)?;
                }
            }
        }
    } else {
        println!("Available modules:");
        for (i, m) in available.iter().enumerate() {
            println!("  {}: {}", i + 1, m);
        }
        print!("Enter module name to enable: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let name = input.trim();
        if !name.is_empty() {
            enable(paths, name, false)?;
        }
    }

    Ok(())
}

pub fn disable_interactive(paths: &ConfigPaths) -> Result<()> {
    let config = load_config(paths)?;

    if config.enabled_modules.is_empty() {
        println!("{}", "No modules are enabled.".yellow());
        return Ok(());
    }

    if which::which("fzf").is_ok() {
        let input = config.enabled_modules.join("\n");
        let output = std::process::Command::new("fzf")
            .args(["--multi", "--prompt=Disable module> "])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(stdin) = child.stdin.take() {
                    let mut stdin = stdin;
                    let _ = stdin.write_all(input.as_bytes());
                }
                child.wait_with_output()
            });

        if let Ok(out) = output {
            let selected = String::from_utf8_lossy(&out.stdout);
            for name in selected.lines() {
                let name = name.trim();
                if !name.is_empty() {
                    disable(paths, name, false)?;
                }
            }
        }
    } else {
        println!("Enabled modules:");
        for (i, m) in config.enabled_modules.iter().enumerate() {
            println!("  {}: {}", i + 1, m);
        }
        print!("Enter module name to disable: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let name = input.trim();
        if !name.is_empty() {
            disable(paths, name, false)?;
        }
    }

    Ok(())
}

pub fn create(paths: &ConfigPaths, path: &str) -> Result<()> {
    let modules_dir = paths.modules_dir();
    std::fs::create_dir_all(&modules_dir)?;

    let module_path = modules_dir.join(format!("{}.yaml", path));

    if module_path.exists() {
        anyhow::bail!("Module '{}' already exists at {:?}", path, module_path);
    }

    // Create parent dirs if nested path
    if let Some(parent) = module_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let template = format!(
        r#"description: {}

packages:
  # Add packages here

# conflicts:
#   - other-module

# post_install_hook: scripts/setup-{}.sh
# hook_behavior: once  # ask | always | once | skip
"#,
        path, path
    );

    std::fs::write(&module_path, template)?;

    println!("{}", format!("✓ Module '{}' created at {:?}", path, module_path).green());
    println!("Edit it with: vcli edit");

    Ok(())
}

pub fn run_hook(paths: &ConfigPaths, name: &str) -> Result<()> {
    let pkg_manager = PackageManager::new(paths.clone());
    let module_path = pkg_manager
        .find_module_path(name)
        .ok_or_else(|| anyhow::anyhow!("Module '{}' not found", name))?;

    let module = crate::config::load_module(&module_path)?;

    if let Some(hook) = module.post_install_hook() {
        let hook_path = if module.is_directory() {
            module.root_dir().join(hook)
        } else {
            paths.config_dir.join(hook)
        };

        if !hook_path.exists() {
            anyhow::bail!("Hook script not found: {:?}", hook_path);
        }

        println!("{}", format!("Running post-install hook for '{}'...", name).blue());
        let status = std::process::Command::new("bash")
            .arg(hook_path)
            .status()?;

        if status.success() {
            println!("{}", "✓ Hook completed".green());
        } else {
            anyhow::bail!("Hook failed with exit code: {}", status.code().unwrap_or(-1));
        }
    } else {
        println!("{}", format!("Module '{}' has no post-install hook.", name).yellow());
    }

    Ok(())
}

fn list_module_names(paths: &ConfigPaths) -> Result<Vec<String>> {
    let modules_dir = paths.modules_dir();
    if !modules_dir.exists() {
        return Ok(Vec::new());
    }

    let mut names = Vec::new();
    for entry in std::fs::read_dir(&modules_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().map(|e| e == "yaml").unwrap_or(false) {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if !matches!(stem, "base" | "declared-packages") && !stem.starts_with("system-packages") {
                names.push(stem.to_string());
            }
        } else if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if !name.starts_with('.') {
                    names.push(name.to_string());
                }
            }
        }
    }
    names.sort();
    Ok(names)
}
