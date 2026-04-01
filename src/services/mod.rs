//! Runit service management for Void Linux.
//!
//! Void Linux uses runit instead of systemd.
//! - Services live in /etc/sv/<service>/
//! - Enabled services are symlinked into /var/service/
//!
//! Enable:   ln -sf /etc/sv/<service> /var/service/
//! Disable:  rm -f /var/service/<service>
//! Start:    sv start <service>
//! Stop:     sv stop <service>
//! Status:   sv status <service>

use anyhow::{Context, Result};
use colored::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

const SV_DIR: &str = "/etc/sv";
const SERVICE_DIR: &str = "/var/service";

/// Service manager for runit
pub struct ServiceManager;

/// Preview of service changes (for dry-run)
#[derive(Debug, Default)]
pub struct ServicesPreview {
    pub services_to_enable: Vec<String>,
    pub services_to_disable: Vec<String>,
    pub services_to_start: Vec<String>,
    pub services_to_stop: Vec<String>,
}

impl ServicesPreview {
    pub fn has_changes(&self) -> bool {
        !self.services_to_enable.is_empty()
            || !self.services_to_disable.is_empty()
            || !self.services_to_start.is_empty()
            || !self.services_to_stop.is_empty()
    }
}

/// Service sync report
#[derive(Debug, Default)]
pub struct ServiceReport {
    pub enabled: Vec<String>,
    pub disabled: Vec<String>,
    pub errors: Vec<String>,
}

impl ServiceReport {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// Services state file structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServicesState {
    #[serde(default)]
    pub enabled: Vec<String>,
    #[serde(default)]
    pub disabled: Vec<String>,
}

/// Load services state from file
pub fn load_services_state(state_file: &Path) -> Result<ServicesState> {
    if !state_file.exists() {
        return Ok(ServicesState::default());
    }
    let content =
        std::fs::read_to_string(state_file).context("Failed to read services state file")?;
    serde_yaml::from_str(&content).context("Failed to parse services state file")
}

/// Save services state to file
pub fn save_services_state(state_file: &Path, state: &ServicesState) -> Result<()> {
    if let Some(parent) = state_file.parent() {
        std::fs::create_dir_all(parent).context("Failed to create state directory")?;
    }
    let yaml = serde_yaml::to_string(state).context("Failed to serialize services state")?;
    std::fs::write(state_file, yaml).context("Failed to write services state file")
}

/// Create updated state from current enabled/disabled lists
pub fn create_updated_state(enabled: &[String], disabled: &[String]) -> ServicesState {
    ServicesState {
        enabled: enabled.to_vec(),
        disabled: disabled.to_vec(),
    }
}

impl ServiceManager {
    /// Check if a service is enabled (symlink exists in /var/service/)
    pub fn is_enabled(service: &str) -> bool {
        Path::new(SERVICE_DIR).join(service).exists()
    }

    /// Check if a service directory exists in /etc/sv/
    pub fn service_exists(service: &str) -> bool {
        Path::new(SV_DIR).join(service).is_dir()
    }

    /// Enable a service (create symlink)
    pub fn enable(service: &str) -> Result<()> {
        let sv_path = Path::new(SV_DIR).join(service);
        let link_path = Path::new(SERVICE_DIR).join(service);

        if !sv_path.is_dir() {
            anyhow::bail!(
                "Service '{}' not found in {}. Is it installed?",
                service,
                SV_DIR
            );
        }

        if link_path.exists() || link_path.is_symlink() {
            // Already enabled
            return Ok(());
        }

        // Need root to create symlink in /var/service/
        let status = std::process::Command::new("ln")
            .args(["-sf", sv_path.to_str().unwrap(), link_path.to_str().unwrap()])
            .status()
            .context(format!("Failed to enable service '{}'", service))?;

        if !status.success() {
            anyhow::bail!("Failed to enable service '{}' (try with sudo)", service);
        }

        Ok(())
    }

    /// Disable a service (remove symlink)
    pub fn disable(service: &str) -> Result<()> {
        let link_path = Path::new(SERVICE_DIR).join(service);

        if !link_path.exists() && !link_path.is_symlink() {
            // Already disabled
            return Ok(());
        }

        let status = std::process::Command::new("rm")
            .args(["-f", link_path.to_str().unwrap()])
            .status()
            .context(format!("Failed to disable service '{}'", service))?;

        if !status.success() {
            anyhow::bail!("Failed to disable service '{}' (try with sudo)", service);
        }

        Ok(())
    }

    /// Get status of a service
    pub fn status(service: &str) -> String {
        let out = std::process::Command::new("sv")
            .args(["status", service])
            .output();

        match out {
            Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
            Err(_) => "unknown".to_string(),
        }
    }

    /// Start a service (sv start <service>)
    pub fn start(service: &str) -> Result<()> {
        let status = std::process::Command::new("sv")
            .args(["start", service])
            .status()
            .context(format!("Failed to start service '{}'", service))?;

        if !status.success() {
            anyhow::bail!("sv start '{}' failed", service);
        }
        Ok(())
    }

    /// Stop a service (sv stop <service>)
    pub fn stop(service: &str) -> Result<()> {
        let status = std::process::Command::new("sv")
            .args(["stop", service])
            .status()
            .context(format!("Failed to stop service '{}'", service))?;

        if !status.success() {
            anyhow::bail!("sv stop '{}' failed", service);
        }
        Ok(())
    }

    /// Get list of currently enabled services (links in /var/service/)
    pub fn get_enabled_services() -> Result<Vec<String>> {
        let service_dir = Path::new(SERVICE_DIR);
        if !service_dir.exists() {
            return Ok(Vec::new());
        }

        let mut services = Vec::new();
        for entry in std::fs::read_dir(service_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_symlink() || path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    services.push(name.to_string());
                }
            }
        }
        services.sort();
        Ok(services)
    }

    /// Preview service changes
    pub fn preview_services(
        to_enable: &[String],
        to_disable: &[String],
        _previous_state: &ServicesState,
    ) -> Result<ServicesPreview> {
        let mut preview = ServicesPreview::default();

        for svc in to_enable {
            if !Self::is_enabled(svc) {
                preview.services_to_enable.push(svc.clone());
                preview.services_to_start.push(svc.clone());
            }
        }

        for svc in to_disable {
            if Self::is_enabled(svc) {
                preview.services_to_disable.push(svc.clone());
                preview.services_to_stop.push(svc.clone());
            }
        }

        Ok(preview)
    }

    /// Sync services based on configuration
    pub fn sync_services(
        to_enable: &[String],
        to_disable: &[String],
        _previous_state: &ServicesState,
    ) -> Result<ServiceReport> {
        let mut report = ServiceReport::default();

        for svc in to_enable {
            if !Self::is_enabled(svc) {
                println!("  {} Enabling service: {}", "→".blue(), svc.cyan());
                match Self::enable(svc) {
                    Ok(_) => {
                        println!("  {} Enabled {}", "✓".green(), svc);
                        report.enabled.push(svc.clone());
                    }
                    Err(e) => {
                        eprintln!("  {} Failed to enable {}: {}", "✗".red(), svc, e);
                        report.errors.push(format!("enable {}: {}", svc, e));
                    }
                }
            } else {
                println!("  {} Service already enabled: {}", "✓".green(), svc);
            }
        }

        for svc in to_disable {
            if Self::is_enabled(svc) {
                println!("  {} Disabling service: {}", "→".blue(), svc.cyan());

                // Stop first
                let _ = Self::stop(svc);

                match Self::disable(svc) {
                    Ok(_) => {
                        println!("  {} Disabled {}", "✓".green(), svc);
                        report.disabled.push(svc.clone());
                    }
                    Err(e) => {
                        eprintln!("  {} Failed to disable {}: {}", "✗".red(), svc, e);
                        report.errors.push(format!("disable {}: {}", svc, e));
                    }
                }
            } else {
                println!(
                    "  {} Service already disabled: {}",
                    "✓".green(),
                    svc
                );
            }
        }

        Ok(report)
    }
}
