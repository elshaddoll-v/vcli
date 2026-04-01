use anyhow::{Context, Result};

use crate::config::{Config, ConfigPaths, FlatpakScope, PackageEntry, PackageType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub name: String,
    pub package_type: PackageType,
}

impl Package {
    pub fn new(name: String, package_type: PackageType) -> Self {
        Self { name, package_type }
    }
}

impl From<&PackageEntry> for Package {
    fn from(entry: &PackageEntry) -> Self {
        Package {
            name: entry.name().to_string(),
            package_type: entry.package_type(),
        }
    }
}

pub struct PackageManager {
    pub paths: ConfigPaths,
}

impl PackageManager {
    pub fn new(paths: ConfigPaths) -> Self {
        Self { paths }
    }

    /// Get all declared packages from config + base + enabled modules
    pub fn get_declared_packages(&self, config: &Config) -> Result<Vec<Package>> {
        let mut packages: Vec<Package> = Vec::new();
        let mut exclude_set: std::collections::HashSet<String> = config.exclude.iter().cloned().collect();

        // Add excludes from config
        for excl in &config.exclude {
            exclude_set.insert(excl.clone());
        }

        // Load base packages
        let base_file = self.paths.base_packages_file();
        if base_file.exists() {
            let base_list = crate::config::load_package_list(&base_file)?;
            for pkg in base_list.packages {
                if !exclude_set.contains(pkg.name()) {
                    packages.push(Package::from(&pkg));
                }
            }
        }

        // Load system-packages (from dcli merge)
        let sys_pkg_file = self
            .paths
            .modules_dir()
            .join(format!("system-packages-{}/packages.yaml", config.host));
        if sys_pkg_file.exists() {
            let sys_list = crate::config::load_package_list(&sys_pkg_file)?;
            for pkg in sys_list.packages {
                if !exclude_set.contains(pkg.name()) {
                    packages.push(Package::from(&pkg));
                }
            }
        }

        // Load declared-packages (from vcli install)
        let decl_file = self.paths.modules_dir().join("declared-packages.yaml");
        if decl_file.exists() {
            let decl_list = crate::config::load_package_list(&decl_file)?;
            for pkg in decl_list.packages {
                if !exclude_set.contains(pkg.name()) {
                    packages.push(Package::from(&pkg));
                }
            }
        }

        // Load host-specific packages
        for pkg in &config.packages {
            if !exclude_set.contains(pkg.name()) {
                packages.push(Package::from(pkg));
            }
        }
        for pkg in &config.additional_packages {
            if !exclude_set.contains(pkg.name()) {
                packages.push(Package::from(pkg));
            }
        }

        // Load enabled modules
        for module_name in &config.enabled_modules {
            let module_path = self.find_module_path(module_name);
            if let Some(path) = module_path {
                match crate::config::load_module(&path) {
                    Ok(module) => {
                        for pkg in module.packages() {
                            if !exclude_set.contains(pkg.name()) {
                                packages.push(Package::from(&pkg));
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to load module '{}': {}", module_name, e);
                    }
                }
            } else {
                log::warn!("Module '{}' not found", module_name);
            }
        }

        // Deduplicate by name+type
        let mut seen = std::collections::HashSet::new();
        packages.retain(|p| seen.insert(p.name.clone()));

        Ok(packages)
    }

    /// Find module path given a name (searches modules directory)
    pub fn find_module_path(&self, module_name: &str) -> Option<std::path::PathBuf> {
        let modules_dir = self.paths.modules_dir();

        // Direct .yaml file
        let yaml_path = modules_dir.join(format!("{}.yaml", module_name));
        if yaml_path.exists() {
            return Some(yaml_path);
        }

        // Direct directory
        let dir_path = modules_dir.join(module_name);
        if dir_path.exists() && dir_path.is_dir() {
            return Some(dir_path);
        }

        // Search subdirectories
        use walkdir::WalkDir;
        for entry in WalkDir::new(&modules_dir)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file()
                && path.extension().and_then(|s| s.to_str()) == Some("yaml")
                && path.file_stem().and_then(|s| s.to_str()) == Some(module_name)
            {
                return Some(path.to_path_buf());
            } else if path.is_dir()
                && path.file_name().and_then(|s| s.to_str()) == Some(module_name)
            {
                return Some(path.to_path_buf());
            }
        }

        None
    }

    /// Get list of manually-installed xbps packages (xbps-query -m)
    /// Returns Vec<(name, version)>
    pub fn get_installed_xbps_packages(&self) -> Result<Vec<(String, String)>> {
        let output = std::process::Command::new("xbps-query")
            .args(["-m"])
            .output()
            .context("Failed to run xbps-query -m. Is xbps installed?")?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            // xbps-query -m output format: "pkgname-version_revision"
            // We need to strip the version suffix to get just the package name.
            // The package name is everything before the last hyphen-followed-by-digit sequence.
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let (name, version) = split_xbps_pkgver(trimmed);
            packages.push((name, version));
        }

        Ok(packages)
    }

    /// Get installed flatpak app IDs
    pub fn get_installed_flatpaks(&self, scope: &str) -> Result<Vec<String>> {
        let output = std::process::Command::new("flatpak")
            .args(["list", "--app", "--columns=application", scope])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                Ok(stdout
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect())
            }
            _ => Ok(Vec::new()),
        }
    }

    /// Install xbps packages (xbps-install -S pkg1 pkg2 ...)
    pub fn install_xbps(&self, packages: &[String], noconfirm: bool) -> Result<()> {
        if packages.is_empty() {
            return Ok(());
        }

        let mut cmd = std::process::Command::new("xbps-install");
        cmd.arg("-S"); // sync repos and install

        if noconfirm {
            cmd.arg("-y");
        }

        cmd.args(packages)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());

        let status = cmd.status().context("Failed to run xbps-install")?;

        if !status.success() {
            anyhow::bail!(
                "xbps-install failed (exit code: {})",
                status.code().unwrap_or(-1)
            );
        }

        Ok(())
    }

    /// Remove xbps packages (xbps-remove -R pkg1 pkg2 ...)
    pub fn remove_xbps(&self, packages: &[String], noconfirm: bool) -> Result<()> {
        if packages.is_empty() {
            return Ok(());
        }

        let mut cmd = std::process::Command::new("xbps-remove");
        cmd.arg("-R"); // remove with recursive orphan cleanup

        if noconfirm {
            cmd.arg("-y");
        }

        cmd.args(packages)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());

        let status = cmd.status().context("Failed to run xbps-remove")?;

        if !status.success() {
            anyhow::bail!(
                "xbps-remove failed (exit code: {})",
                status.code().unwrap_or(-1)
            );
        }

        Ok(())
    }

    /// Install a flatpak
    pub fn install_flatpak(&self, app_id: &str, scope: &FlatpakScope) -> Result<()> {
        let status = std::process::Command::new("flatpak")
            .args(["install", "-y", scope.as_flag(), "flathub", app_id])
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .context(format!("Failed to install flatpak: {}", app_id))?;

        if !status.success() {
            anyhow::bail!("flatpak install failed for {}", app_id);
        }
        Ok(())
    }

    /// Remove a flatpak
    pub fn remove_flatpak(&self, app_id: &str, scope: &FlatpakScope) -> Result<()> {
        let status = std::process::Command::new("flatpak")
            .args(["uninstall", "-y", scope.as_flag(), app_id])
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .context(format!("Failed to remove flatpak: {}", app_id))?;

        if !status.success() {
            anyhow::bail!("flatpak uninstall failed for {}", app_id);
        }
        Ok(())
    }

    /// Check if a package is installed via xbps
    pub fn is_xbps_installed(&self, pkg_name: &str) -> bool {
        std::process::Command::new("xbps-query")
            .args(["-S", pkg_name])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Check if a package exists in the repos
    pub fn xbps_pkg_exists(&self, pkg_name: &str) -> bool {
        let out = std::process::Command::new("xbps-query")
            .args(["-Rs", pkg_name])
            .output();
        match out {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.lines().any(|l| {
                    let parts: Vec<&str> = l.splitn(2, ' ').collect();
                    if parts.len() >= 2 {
                        split_xbps_pkgver(parts[1].trim()).0 == pkg_name
                    } else {
                        false
                    }
                })
            }
            Err(_) => false,
        }
    }
}

/// Get installed package version via xbps-query
pub fn get_package_version(pkg_name: &str) -> Option<String> {
    let output = std::process::Command::new("xbps-query")
        .args(["-p", "pkgver", pkg_name])
        .output()
        .ok()?;

    if output.status.success() {
        let ver_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // pkgver format: "name-version_revision" — strip the name prefix
        let (_, version) = split_xbps_pkgver(&ver_str);
        if !version.is_empty() {
            return Some(version);
        }
    }
    None
}

/// Get installed flatpak version
pub fn get_flatpak_version(app_id: &str) -> Option<String> {
    let output = std::process::Command::new("flatpak")
        .args(["info", app_id])
        .output()
        .ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.trim_start().starts_with("Version:") {
                return line.split(':').nth(1).map(|v| v.trim().to_string());
            }
        }
    }
    None
}

/// Split an xbps pkgver string (e.g., "neovim-0.10.0_1") into (name, version)
/// xbps package names can contain hyphens, so we find the last hyphen
/// followed by a digit to split name from version.
pub fn split_xbps_pkgver(pkgver: &str) -> (String, String) {
    // Find the last '-' that is followed by a digit (start of version)
    let bytes = pkgver.as_bytes();
    let mut split_pos = pkgver.len();

    for i in (0..pkgver.len()).rev() {
        if bytes[i] == b'-' {
            if i + 1 < pkgver.len() && bytes[i + 1].is_ascii_digit() {
                split_pos = i;
                break;
            }
        }
    }

    if split_pos == pkgver.len() {
        // No version found
        return (pkgver.to_string(), String::new());
    }

    let name = pkgver[..split_pos].to_string();
    let version = pkgver[split_pos + 1..].to_string();
    (name, version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_xbps_pkgver_simple() {
        let (name, ver) = split_xbps_pkgver("neovim-0.10.0_1");
        assert_eq!(name, "neovim");
        assert_eq!(ver, "0.10.0_1");
    }

    #[test]
    fn test_split_xbps_pkgver_hyphenated() {
        let (name, ver) = split_xbps_pkgver("python3-pip-24.0_1");
        assert_eq!(name, "python3-pip");
        assert_eq!(ver, "24.0_1");
    }

    #[test]
    fn test_split_xbps_pkgver_no_version() {
        let (name, ver) = split_xbps_pkgver("just-a-name");
        assert_eq!(name, "just-a-name");
        assert_eq!(ver, "");
    }
}
