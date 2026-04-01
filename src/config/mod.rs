use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Main configuration structure for vcli
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Hostname of the current machine
    pub host: String,

    /// Optional description of this host
    #[serde(default)]
    pub description: String,

    /// Import additional config files (relative to void-config root)
    #[serde(default)]
    pub import: Vec<String>,

    /// List of enabled module names
    #[serde(default)]
    pub enabled_modules: Vec<String>,

    /// Host-specific packages to install
    #[serde(default)]
    pub packages: Vec<PackageEntry>,

    /// Packages to exclude from base or modules
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Additional packages (backwards compatibility)
    #[serde(default)]
    pub additional_packages: Vec<PackageEntry>,

    /// Flatpak installation scope: "user" or "system"
    #[serde(default = "default_flatpak_scope")]
    pub flatpak_scope: FlatpakScope,

    /// Automatically prune packages during sync (default: false)
    #[serde(default)]
    pub auto_prune: bool,

    /// Module processing mode: "parallel" (default) or "sequential"
    #[serde(default = "default_module_processing")]
    pub module_processing: ModuleProcessing,

    /// Install packages one-at-a-time in strict order (default: false)
    #[serde(default)]
    pub strict_package_order: bool,

    /// Configuration backup settings
    #[serde(default)]
    pub config_backups: ConfigBackupsSettings,

    /// System services configuration (runit)
    #[serde(default)]
    pub services: ServicesConfig,

    /// Update hooks configuration
    #[serde(default)]
    pub update_hooks: UpdateHooksConfig,

    /// Editor to use for config file editing (falls back to $EDITOR)
    #[serde(default)]
    pub editor: Option<String>,

    /// Automatically commit changes to git after successful sync (default: false)
    #[serde(default)]
    pub auto_commit: bool,
}

impl Config {
    /// Merge another config into this one (imports)
    pub fn merge(&mut self, other: Config) {
        for module in other.enabled_modules {
            if !self.enabled_modules.contains(&module) {
                self.enabled_modules.push(module);
            }
        }
        self.packages.extend(other.packages);
        self.additional_packages.extend(other.additional_packages);
        for exclude in other.exclude {
            if !self.exclude.contains(&exclude) {
                self.exclude.push(exclude);
            }
        }
        for service in other.services.enabled {
            if !self.services.enabled.contains(&service) {
                self.services.enabled.push(service);
            }
        }
        for service in other.services.disabled {
            if !self.services.disabled.contains(&service) {
                self.services.disabled.push(service);
            }
        }
        if self.description.is_empty() && !other.description.is_empty() {
            self.description = other.description;
        }
    }
}

fn default_flatpak_scope() -> FlatpakScope {
    FlatpakScope::User
}

fn default_module_processing() -> ModuleProcessing {
    ModuleProcessing::Parallel
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FlatpakScope {
    User,
    System,
}

impl FlatpakScope {
    pub fn as_flag(&self) -> &str {
        match self {
            FlatpakScope::User => "--user",
            FlatpakScope::System => "--system",
        }
    }
}

/// Hook execution user configuration
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum RunHooksAsUser {
    Bool(bool),
    Username(String),
}

impl Default for RunHooksAsUser {
    fn default() -> Self {
        RunHooksAsUser::Bool(false)
    }
}

impl RunHooksAsUser {
    pub fn username(&self) -> Option<String> {
        match self {
            RunHooksAsUser::Bool(false) => None,
            RunHooksAsUser::Bool(true) => std::env::var("USER").ok(),
            RunHooksAsUser::Username(s) if s.is_empty() => None,
            RunHooksAsUser::Username(s) => Some(s.clone()),
        }
    }
}

impl<'de> Deserialize<'de> for RunHooksAsUser {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let value = serde_yaml::Value::deserialize(deserializer)?;
        if let Some(b) = value.as_bool() {
            return Ok(RunHooksAsUser::Bool(b));
        }
        if let Some(s) = value.as_str() {
            return Ok(RunHooksAsUser::Username(s.to_string()));
        }
        Err(D::Error::custom(
            "run_hooks_as_user must be a boolean or a username string",
        ))
    }
}

/// Configuration backup settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigBackupsSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_max_backups")]
    pub max_backups: u32,
}

impl Default for ConfigBackupsSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_backups: 5,
        }
    }
}

/// Update hooks configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateHooksConfig {
    #[serde(default)]
    pub pre_update: Option<String>,
    #[serde(default)]
    pub post_update: Option<String>,
    #[serde(default = "default_hook_behavior")]
    pub behavior: String,
    #[serde(default)]
    pub run_as_user: bool,
}

impl Default for UpdateHooksConfig {
    fn default() -> Self {
        Self {
            pre_update: None,
            post_update: None,
            behavior: "ask".to_string(),
            run_as_user: false,
        }
    }
}

/// System services configuration (runit on Void Linux)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesConfig {
    /// Services to enable (symlink into /var/service/)
    #[serde(default)]
    pub enabled: Vec<String>,

    /// Services to disable (remove symlink from /var/service/)
    #[serde(default)]
    pub disabled: Vec<String>,
}

impl Default for ServicesConfig {
    fn default() -> Self {
        Self {
            enabled: Vec::new(),
            disabled: Vec::new(),
        }
    }
}

/// Module processing mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModuleProcessing {
    Parallel,
    Sequential,
}

/// Package entry: simple string or object with type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PackageEntry {
    Simple(String),
    WithType {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        r#type: Option<PackageType>,
    },
}

impl PackageEntry {
    pub fn name(&self) -> &str {
        match self {
            PackageEntry::Simple(name) => {
                if let Some(stripped) = name.strip_prefix("flatpak:") {
                    stripped
                } else {
                    name
                }
            }
            PackageEntry::WithType { name, .. } => name,
        }
    }

    pub fn package_type(&self) -> PackageType {
        match self {
            PackageEntry::Simple(name) => {
                if name.starts_with("flatpak:") {
                    PackageType::Flatpak
                } else {
                    PackageType::Xbps
                }
            }
            PackageEntry::WithType { r#type, .. } => r#type.clone().unwrap_or(PackageType::Xbps),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PackageType {
    /// Regular xbps package (xbps-install)
    Xbps,
    /// Flatpak package
    Flatpak,
}

/// Package list file (base.yaml, host files, modules)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageList {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub packages: Vec<PackageEntry>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub conflicts: Vec<String>,
    #[serde(default)]
    pub pre_install_hook: Option<String>,
    #[serde(default)]
    pub post_install_hook: Option<String>,
    #[serde(default = "default_hook_behavior", alias = "hooks_behavior")]
    pub hook_behavior: String,
    #[serde(default)]
    pub pre_hook_behavior: Option<String>,
    #[serde(default)]
    pub post_hook_behavior: Option<String>,
    #[serde(default)]
    pub run_hooks_as_user: RunHooksAsUser,
    #[serde(default)]
    pub post_disable_hook: Option<String>,
    #[serde(default)]
    pub post_disable_behavior: Option<String>,
}

/// Module structure: legacy (single .yaml) or directory-based
#[derive(Debug, Clone)]
pub enum ModuleStructure {
    Legacy { path: PathBuf, content: PackageList },
    Directory(DirectoryModule),
}

impl ModuleStructure {
    pub fn description(&self) -> &str {
        match self {
            ModuleStructure::Legacy { content, .. } => &content.description,
            ModuleStructure::Directory(dir) => &dir.manifest.description,
        }
    }

    pub fn conflicts(&self) -> &[String] {
        match self {
            ModuleStructure::Legacy { content, .. } => &content.conflicts,
            ModuleStructure::Directory(dir) => &dir.manifest.conflicts,
        }
    }

    pub fn pre_install_hook(&self) -> Option<&str> {
        match self {
            ModuleStructure::Legacy { content, .. } => content.pre_install_hook.as_deref(),
            ModuleStructure::Directory(dir) => dir.manifest.pre_install_hook.as_deref(),
        }
    }

    pub fn post_install_hook(&self) -> Option<&str> {
        match self {
            ModuleStructure::Legacy { content, .. } => content.post_install_hook.as_deref(),
            ModuleStructure::Directory(dir) => dir.manifest.post_install_hook.as_deref(),
        }
    }

    pub fn post_disable_hook(&self) -> Option<&str> {
        match self {
            ModuleStructure::Legacy { content, .. } => content.post_disable_hook.as_deref(),
            ModuleStructure::Directory(dir) => dir.manifest.post_disable_hook.as_deref(),
        }
    }

    pub fn hook_behavior(&self) -> &str {
        match self {
            ModuleStructure::Legacy { content, .. } => &content.hook_behavior,
            ModuleStructure::Directory(dir) => &dir.manifest.hook_behavior,
        }
    }

    pub fn pre_hook_behavior(&self) -> &str {
        match self {
            ModuleStructure::Legacy { content, .. } => content
                .pre_hook_behavior
                .as_deref()
                .unwrap_or(&content.hook_behavior),
            ModuleStructure::Directory(dir) => dir
                .manifest
                .pre_hook_behavior
                .as_deref()
                .unwrap_or(&dir.manifest.hook_behavior),
        }
    }

    pub fn post_hook_behavior(&self) -> &str {
        match self {
            ModuleStructure::Legacy { content, .. } => content
                .post_hook_behavior
                .as_deref()
                .unwrap_or(&content.hook_behavior),
            ModuleStructure::Directory(dir) => dir
                .manifest
                .post_hook_behavior
                .as_deref()
                .unwrap_or(&dir.manifest.hook_behavior),
        }
    }

    pub fn packages(&self) -> Vec<PackageEntry> {
        match self {
            ModuleStructure::Legacy { content, .. } => content.packages.clone(),
            ModuleStructure::Directory(dir) => {
                let mut all = Vec::new();
                for list in &dir.package_lists {
                    all.extend(list.packages.clone());
                }
                all
            }
        }
    }

    pub fn root_dir(&self) -> PathBuf {
        match self {
            ModuleStructure::Legacy { path, .. } => path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from(".")),
            ModuleStructure::Directory(dir) => dir.root.clone(),
        }
    }

    pub fn is_directory(&self) -> bool {
        matches!(self, ModuleStructure::Directory(_))
    }

    pub fn run_hooks_as_user(&self) -> Option<String> {
        match self {
            ModuleStructure::Legacy { content, .. } => content.run_hooks_as_user.username(),
            ModuleStructure::Directory(dir) => dir.manifest.run_hooks_as_user.username(),
        }
    }
}

/// Directory-based module
#[derive(Debug, Clone)]
pub struct DirectoryModule {
    pub root: PathBuf,
    pub manifest: ModuleManifest,
    pub package_lists: Vec<PackageList>,
    pub package_file_paths: Vec<PathBuf>,
    pub scripts_dir: Option<PathBuf>,
    pub dotfiles_dir: Option<PathBuf>,
}

/// Module manifest (module.yaml in directory-based modules)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleManifest {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub conflicts: Vec<String>,
    #[serde(default)]
    pub pre_install_hook: Option<String>,
    #[serde(default)]
    pub post_install_hook: Option<String>,
    #[serde(default = "default_hook_behavior", alias = "hooks_behavior")]
    pub hook_behavior: String,
    #[serde(default)]
    pub pre_hook_behavior: Option<String>,
    #[serde(default)]
    pub post_hook_behavior: Option<String>,
    #[serde(default)]
    pub run_hooks_as_user: RunHooksAsUser,
    #[serde(default)]
    pub post_disable_hook: Option<String>,
    #[serde(default)]
    pub post_disable_behavior: Option<String>,
    #[serde(default)]
    pub package_files: Vec<String>,
    #[serde(default)]
    pub dotfiles_sync: Option<bool>,
    #[serde(default)]
    pub dotfiles: Vec<DotfileEntry>,
}

/// Dotfile entry with source/target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DotfileEntry {
    pub source: String,
    pub target: String,
}

fn default_hook_behavior() -> String {
    "ask".to_string()
}

impl Default for PackageList {
    fn default() -> Self {
        Self {
            description: String::new(),
            packages: Vec::new(),
            exclude: Vec::new(),
            conflicts: Vec::new(),
            pre_install_hook: None,
            post_install_hook: None,
            hook_behavior: "ask".to_string(),
            pre_hook_behavior: None,
            post_hook_behavior: None,
            run_hooks_as_user: RunHooksAsUser::Bool(false),
            post_disable_hook: None,
            post_disable_behavior: None,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_max_backups() -> u32 {
    5
}

/// Configuration paths
#[derive(Debug, Clone)]
pub struct ConfigPaths {
    pub config_dir: PathBuf,
    pub config_file: PathBuf,
    pub packages_dir: PathBuf,
    pub state_dir: PathBuf,
    pub state_file: PathBuf,
    pub hooks_state_file: PathBuf,
    pub services_state_file: PathBuf,
    pub config_backups_dir: PathBuf,
}

impl ConfigPaths {
    pub fn new() -> Result<Self> {
        let config_dir = if let Ok(custom) = std::env::var("VOID_CONFIG_DIR") {
            PathBuf::from(custom)
        } else {
            let home = std::env::var("HOME").context("HOME environment variable not set")?;
            PathBuf::from(home).join(".config/void-config")
        };

        let config_file = config_dir.join("config.yaml");
        let packages_dir = config_dir.join("packages");
        let state_dir = config_dir.join("state");
        let state_file = state_dir.join("installed.yaml");
        let hooks_state_file = state_dir.join("hooks-executed.yaml");
        let services_state_file = state_dir.join("services-state.yaml");
        let config_backups_dir = state_dir.join("config-backups");

        Ok(Self {
            config_dir,
            config_file,
            packages_dir,
            state_dir,
            state_file,
            hooks_state_file,
            services_state_file,
            config_backups_dir,
        })
    }

    pub fn hosts_dir(&self) -> PathBuf {
        let new_path = self.config_dir.join("hosts");
        if new_path.exists() {
            new_path
        } else {
            self.packages_dir.join("hosts")
        }
    }

    pub fn modules_dir(&self) -> PathBuf {
        let new_path = self.config_dir.join("modules");
        if new_path.exists() {
            new_path
        } else {
            self.packages_dir.join("modules")
        }
    }

    pub fn base_packages_file(&self) -> PathBuf {
        let new_path = self.config_dir.join("modules").join("base.yaml");
        if new_path.exists() {
            new_path
        } else {
            self.packages_dir.join("base.yaml")
        }
    }

    pub fn host_packages_file(&self, hostname: &str) -> PathBuf {
        let yaml_filename = format!("{}.yaml", hostname);
        let new_yaml_path = self.config_dir.join("hosts").join(&yaml_filename);
        if new_yaml_path.exists() {
            return new_yaml_path;
        }
        let old_yaml_path = self.packages_dir.join("hosts").join(&yaml_filename);
        if old_yaml_path.exists() {
            return old_yaml_path;
        }
        new_yaml_path
    }
}

impl Default for ConfigPaths {
    fn default() -> Self {
        Self::new().expect("Failed to create default config paths")
    }
}

/// Resolve the effective config file path
pub fn resolve_config_path(paths: &ConfigPaths) -> Result<PathBuf> {
    if !paths.config_file.exists() {
        anyhow::bail!(
            "Config file not found: {:?}\nRun 'vcli init' to create a new configuration.",
            paths.config_file
        );
    }

    let content = std::fs::read_to_string(&paths.config_file).context(format!(
        "Failed to read config file: {:?}",
        paths.config_file
    ))?;

    if is_pointer_config_raw(&content) {
        let config: Config =
            serde_yaml::from_str(&content).context("Failed to parse config.yaml")?;
        let host_file = paths.host_packages_file(&config.host);
        if !host_file.exists() {
            anyhow::bail!(
                "Host file not found: {:?}\nconfig.yaml is a pointer but host file doesn't exist",
                host_file
            );
        }
        return Ok(host_file);
    }

    Ok(paths.config_file.clone())
}

fn is_pointer_config_raw(yaml_content: &str) -> bool {
    let value: serde_yaml::Value = match serde_yaml::from_str(yaml_content) {
        Ok(v) => v,
        Err(_) => return false,
    };
    if let serde_yaml::Value::Mapping(map) = value {
        if map.len() == 1 && map.contains_key(&serde_yaml::Value::String("host".to_string())) {
            return true;
        }
    }
    false
}

/// Load main configuration file
pub fn load_config(paths: &ConfigPaths) -> Result<Config> {
    let config_path = resolve_config_path(paths)?;

    let content = std::fs::read_to_string(&config_path)
        .context(format!("Failed to read config file: {:?}", config_path))?;

    let mut config: Config = serde_yaml::from_str(&content)
        .context(format!("Failed to parse config: {:?}", config_path))?;

    if !config.import.is_empty() {
        let import_list = config.import.clone();
        for import_path in import_list {
            let full_path = if Path::new(&import_path).is_absolute() {
                PathBuf::from(&import_path)
            } else {
                paths.config_dir.join(&import_path)
            };

            if !full_path.exists() {
                eprintln!("⚠️  Warning: Import file not found, skipping: {:?}", import_path);
                continue;
            }

            let imported_content = std::fs::read_to_string(&full_path)
                .context(format!("Failed to read import: {:?}", full_path))?;
            let imported: Config = serde_yaml::from_str(&imported_content)
                .context(format!("Failed to parse import: {:?}", full_path))?;
            config.merge(imported);
        }
    }

    Ok(config)
}

/// Load a package list file
pub fn load_package_list<P: AsRef<Path>>(path: P) -> Result<PackageList> {
    let content = std::fs::read_to_string(path.as_ref())
        .context(format!("Failed to read package list: {:?}", path.as_ref()))?;
    serde_yaml::from_str(&content).context("Failed to parse package list YAML")
}

/// Load a module (detects legacy vs directory format automatically)
pub fn load_module<P: AsRef<Path>>(path: P) -> Result<ModuleStructure> {
    let path = path.as_ref();

    if path.is_file() {
        let content = load_package_list(path)?;
        Ok(ModuleStructure::Legacy {
            path: path.to_path_buf(),
            content,
        })
    } else if path.is_dir() {
        let yaml_manifest_path = path.join("module.yaml");

        let manifest: ModuleManifest = if yaml_manifest_path.exists() {
            let manifest_content = std::fs::read_to_string(&yaml_manifest_path).context(
                format!("Failed to read module.yaml: {:?}", yaml_manifest_path),
            )?;
            serde_yaml::from_str(&manifest_content).context("Failed to parse module.yaml")?
        } else {
            let discovered_files = discover_package_files(path)?;
            if discovered_files.is_empty() {
                anyhow::bail!(
                    "Directory module must contain 'module.yaml' or package YAML files: {:?}",
                    path
                );
            }
            let module_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            ModuleManifest {
                description: format!("Module: {}", module_name),
                package_files: Vec::new(),
                dotfiles: Vec::new(),
                dotfiles_sync: None,
                conflicts: Vec::new(),
                pre_install_hook: None,
                post_install_hook: None,
                hook_behavior: "ask".to_string(),
                pre_hook_behavior: None,
                post_hook_behavior: None,
                run_hooks_as_user: RunHooksAsUser::Bool(false),
                post_disable_hook: None,
                post_disable_behavior: None,
            }
        };

        let package_file_paths = if manifest.package_files.is_empty() {
            discover_package_files(path)?
        } else {
            manifest.package_files.iter().map(|f| path.join(f)).collect()
        };

        let mut package_lists = Vec::new();
        for pkg_path in &package_file_paths {
            if !pkg_path.exists() {
                anyhow::bail!("Package file not found: {:?}", pkg_path);
            }
            let pkg_list = load_package_list(pkg_path)?;
            package_lists.push(pkg_list);
        }

        let scripts_dir = path.join("scripts");
        let scripts_dir = if scripts_dir.exists() && scripts_dir.is_dir() {
            Some(scripts_dir)
        } else {
            None
        };

        let dotfiles_dir = path.join("dotfiles");
        let dotfiles_dir = if dotfiles_dir.exists() && dotfiles_dir.is_dir() {
            Some(dotfiles_dir)
        } else {
            None
        };

        Ok(ModuleStructure::Directory(DirectoryModule {
            root: path.to_path_buf(),
            manifest,
            package_lists,
            package_file_paths,
            scripts_dir,
            dotfiles_dir,
        }))
    } else {
        anyhow::bail!("Module path is neither a file nor directory: {:?}", path)
    }
}

fn discover_package_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).context(format!("Failed to read directory: {:?}", dir))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && path.extension().map(|s| s == "yaml").unwrap_or(false)
            && path.file_name() != Some(std::ffi::OsStr::new("module.yaml"))
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Validation result for modules
#[derive(Debug, Clone)]
pub struct ModuleValidationResult {
    pub module_name: String,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ModuleValidationResult {
    pub fn new(module_name: String) -> Self {
        Self {
            module_name,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn is_clean(&self) -> bool {
        self.errors.is_empty() && self.warnings.is_empty()
    }
}

/// Validate a module structure
pub fn validate_module(module: &ModuleStructure, module_name: &str) -> ModuleValidationResult {
    let mut result = ModuleValidationResult::new(module_name.to_string());

    match module {
        ModuleStructure::Legacy { content, .. } => {
            if content.description.is_empty() {
                result.warnings.push("Module has no description".to_string());
            }
            let mut seen = std::collections::HashSet::new();
            for pkg in &content.packages {
                let name = pkg.name();
                if !seen.insert(name) {
                    result.errors.push(format!("Duplicate package: {}", name));
                }
            }
        }
        ModuleStructure::Directory(dir) => {
            if dir.manifest.description.is_empty() {
                result.warnings.push("Module has no description".to_string());
            }
            if dir.package_lists.is_empty() {
                result
                    .warnings
                    .push("No package files found (module has no packages)".to_string());
            }
            for (idx, pkg_list) in dir.package_lists.iter().enumerate() {
                if pkg_list.packages.is_empty() {
                    let file_name = dir
                        .package_file_paths
                        .get(idx)
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");
                    result
                        .warnings
                        .push(format!("Package file '{}' is empty", file_name));
                }
            }
            if let Some(hook) = &dir.manifest.post_install_hook {
                if !hook.is_empty() {
                    let hook_path = dir.root.join(hook);
                    if !hook_path.exists() {
                        result.errors.push(format!(
                            "post_install_hook script not found: {} (resolved to {:?})",
                            hook, hook_path
                        ));
                    }
                }
            }
            if let Some(hook) = &dir.manifest.pre_install_hook {
                if !hook.is_empty() {
                    let hook_path = dir.root.join(hook);
                    if !hook_path.exists() {
                        result.errors.push(format!(
                            "pre_install_hook script not found: {} (resolved to {:?})",
                            hook, hook_path
                        ));
                    }
                }
            }
        }
    }

    result
}

/// Resolve the editor to use
pub fn resolve_editor(config: &Config) -> Result<String> {
    if let Some(ref editor) = config.editor {
        return Ok(editor.clone());
    }
    if let Ok(editor) = std::env::var("EDITOR") {
        if !editor.trim().is_empty() {
            return Ok(editor);
        }
    }
    if which::which("vi").is_ok() {
        return Ok("vi".to_string());
    }
    if which::which("nano").is_ok() {
        return Ok("nano".to_string());
    }
    anyhow::bail!(
        "No editor found. Please set 'editor' in your config.yaml or set the $EDITOR environment variable."
    )
}
