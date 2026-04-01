mod commands;
mod config;
mod dotfiles;
mod module;
mod package;
mod progress;
mod services;

use anyhow::Result;
use clap::{Parser, Subcommand};
use config::ConfigPaths;

/// Print a helpful error and exit if a root-required command runs without root.
/// sudo is handled by the fish function in ~/.config/fish/functions/vcli.fish
/// which preserves HOME correctly. The binary itself never calls sudo.
fn check_root(cmd: &Commands) {
    if !needs_root(cmd) { return; }
    if unsafe { libc::geteuid() } == 0 { return; }

    eprintln!("{}", "\x1b[31m✗ This command requires root.\x1b[0m");
    eprintln!();
    eprintln!("Run with:");
    eprintln!("  sudo -E vcli {}", std::env::args().skip(1).collect::<Vec<_>>().join(" "));
    eprintln!();
    eprintln!("Or set up the fish function (do this once):");
    eprintln!("  vcli desktop i3   # creates ~/.config/fish/functions/vcli.fish");
    eprintln!("  # Then just type 'vcli sync' — fish handles sudo automatically");
    std::process::exit(1);
}

fn needs_root(cmd: &Commands) -> bool {
    matches!(cmd,
        Commands::Sync { .. }
        | Commands::Install { .. }
        | Commands::Add { .. }
        | Commands::Remove { .. }
        | Commands::Update { .. }
        | Commands::Orphans { remove: true, .. }
    )
}

#[derive(Parser)]
#[command(name = "vcli")]
#[command(author, version, about = "Declarative package manager for Void Linux", long_about = None)]
struct Cli {
    #[arg(short, long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize void-config directory
    Init { #[arg(short='b', long="bootstrap")] bootstrap: bool },

    /// Install package(s) and track in config
    Install { #[arg(required=true, num_args=1..)] packages: Vec<String> },

    /// Shorthand for install
    Add { #[arg(required=true, num_args=1..)] packages: Vec<String> },

    /// Remove package(s) from config and system
    Remove { #[arg(required=true, num_args=1..)] packages: Vec<String> },

    /// Show config and sync status
    Status,

    /// Sync system to match config (packages + services only, not dotfiles)
    Sync {
        #[arg(short, long)] dry_run: bool,
        #[arg(long)] prune: bool,
        #[arg(long)] force: bool,
        #[arg(long)] no_backup: bool,
        #[arg(long)] no_hooks: bool,
        #[arg(long)] auto_commit: bool,
    },

    /// Module management
    Module { #[command(subcommand)] action: ModuleAction },

    /// System update (xbps-install -Su)
    Update { #[arg(long)] no_backup: bool, #[arg(long)] no_hooks: bool },

    /// Capture installed packages into config
    Merge { #[arg(short, long)] dry_run: bool, #[arg(long)] services: bool },

    /// Find where a package is declared
    Find { package: String },

    /// Stop tracking a package (keep installed)
    Forget { package: String },

    /// Validate config integrity
    Validate { #[arg(long)] check_packages: bool },

    /// Check all declared package names exist in xbps repos
    Check,

    /// List all declared packages with install status
    List,

    /// Show package info
    Info { package: String },

    /// Show what depends on a package
    Why { package: String },

    /// Show packages with available updates
    Outdated,

    /// Show orphaned packages (auto-installed, nothing needs them)
    Orphans { #[arg(long)] remove: bool, #[arg(short, long)] json: bool },

    /// Full system health check
    Doctor,

    /// Show vcli operation history
    Log { #[arg(short='n', long, default_value="30")] lines: usize },

    // ── Desktop ───────────────────────────────────────────────────────────
    /// Scaffold complete desktop: sxhkdrc, xprofile, scripts, fish function
    Desktop {
        /// WM to use: i3 (default), dwm, bspwm, awesome, openbox
        #[arg(default_value = "i3")]
        wm: String,
    },

    /// Manage environment variables (TERMINAL, BROWSER, EDITOR, etc.)
    Env { #[command(subcommand)] action: EnvAction },

    /// Apply wallpaper and generate theme colors (wallust or pywal)
    Theme {
        /// Path to wallpaper (opens fzf picker from ~/Pictures/wallpapers/ if omitted)
        wallpaper: Option<String>,
        /// Only reload apps (dunst, alacritty) without changing wallpaper
        #[arg(long)]
        reload: bool,
        /// Show current wallpaper
        #[arg(long)]
        current: bool,
    },

    /// Manage desktop rices (complete WM/dotfile configurations)
    Rice { #[command(subcommand)] action: RiceAction },

    // ── Maintenance ───────────────────────────────────────────────────────
    /// Post-install hook management
    Hooks { #[command(subcommand)] action: HooksAction },

    /// Git repository management
    Repo { #[command(subcommand)] action: RepoAction },

    /// Update vcli itself from source
    SelfUpdate,

    /// Interactive package search with fzf (requires fzf)
    Search,

    /// Edit config files interactively
    Edit,

    /// Backup config to state/config-backups/
    SaveConfig,

    /// Restore config from a backup
    RestoreConfig { backup: Option<String> },

    /// Generate shell completions (fish, bash, zsh)
    Completions { shell: String },
}

#[derive(Subcommand)]
enum EnvAction {
    /// Show all env vars, or one: vcli env get TERMINAL
    Get { key: Option<String> },
    /// Set a var: vcli env set TERMINAL foot
    Set { key: String, value: String },
    /// Remove a var: vcli env unset LOCKER
    Unset { key: String },
    /// Create ~/.config/environment.sh from template
    Init,
    /// Open ~/.config/environment.sh in $EDITOR
    Edit,
}

#[derive(Subcommand)]
enum RiceAction {
    /// List all rices and show which is active
    List,
    /// Apply a rice (symlinks dotfiles, installs packages, applies theme)
    Apply { name: String, #[arg(long)] dry_run: bool, #[arg(long)] force: bool },
    /// Create a new rice scaffold with manifest.toml
    Create { name: String },
    /// Show currently active rice
    Current,
}

#[derive(Subcommand)]
enum HooksAction {
    List,
    Reset { module: String, #[arg(long)] pre: bool },
    Skip  { module: String, #[arg(long)] pre: bool },
    Run   { module: String, #[arg(long)] pre: bool },
}

#[derive(Subcommand)]
enum RepoAction {
    Init, Clone, Push, Pull, Status,
}

#[derive(Subcommand)]
enum ModuleAction {
    List,
    Enable  { name: Option<String> },
    Disable { name: Option<String> },
    Create  { path: String },
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let cli = Cli::parse();

    // Check root BEFORE creating ConfigPaths so error message is clear
    check_root(&cli.command);

    let paths = ConfigPaths::new()?;

    // Log the operation
    if paths.state_dir.exists() {
        let op = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
        commands::log::append(&paths.state_dir, "cmd", &op);
    }

    match cli.command {
        Commands::Init { bootstrap } => commands::init::run(&paths, bootstrap)?,

        Commands::Install { packages } | Commands::Add { packages } => {
            for pkg in &packages {
                commands::log::append(&paths.state_dir, "install", pkg);
                commands::simple::install(pkg, &paths)?;
            }
        }
        Commands::Remove { packages } => {
            for pkg in &packages {
                commands::log::append(&paths.state_dir, "remove", pkg);
                commands::simple::remove(pkg, &paths)?;
            }
        }

        Commands::Status => commands::status::run(&paths, cli.json)?,

        Commands::Module { action } => match action {
            ModuleAction::List => commands::module::list(&paths, cli.json)?,
            ModuleAction::Enable { name } => {
                if let Some(name) = name {
                    commands::log::append(&paths.state_dir, "module-enable", &name);
                    commands::module::enable(&paths, &name, cli.json)?;
                } else { commands::module::enable_interactive(&paths)?; }
            }
            ModuleAction::Disable { name } => {
                if let Some(name) = name {
                    commands::log::append(&paths.state_dir, "module-disable", &name);
                    commands::module::disable(&paths, &name, cli.json)?;
                } else { commands::module::disable_interactive(&paths)?; }
            }
            ModuleAction::Create { path } => commands::module::create(&paths, &path)?,
        },

        Commands::Update { no_backup, no_hooks } => {
            commands::log::append(&paths.state_dir, "update", "system");
            commands::update::run(&paths, no_backup, no_hooks)?;
        }

        Commands::Merge { dry_run, services } => commands::merge::run(&paths, dry_run, services)?,
        Commands::Find { package } => commands::find::run(&paths, &package, cli.json)?,
        Commands::Forget { package } => commands::forget::run(&paths, &package)?,

        Commands::Sync { dry_run, prune, force, no_backup, no_hooks, auto_commit } => {
            if !dry_run { commands::log::append(&paths.state_dir, "sync", ""); }
            commands::sync::run(
                &paths, dry_run, prune, force, no_backup, no_hooks,
                false, // force_dotfiles removed — dotfiles handled by rice only
                cli.json, auto_commit,
            )?;
        }

        Commands::Validate { check_packages } => commands::validate::run(&paths, check_packages, cli.json)?,
        Commands::Check => commands::check::run(&paths)?,
        Commands::List => commands::list::run(&paths, cli.json)?,
        Commands::Info { package } => commands::info::run(&package)?,
        Commands::Why { package } => commands::why::run(&package)?,
        Commands::Outdated => commands::outdated::run(cli.json)?,
        Commands::Orphans { remove, json } => commands::orphans::run(&paths, remove, json)?,
        Commands::Doctor => commands::doctor::run(&paths)?,
        Commands::Log { lines } => commands::log::run(&paths.state_dir, lines)?,

        // ── Desktop ──────────────────────────────────────────────────────
        Commands::Desktop { wm } => commands::desktop::setup(&wm, &paths)?,

        Commands::Env { action } => match action {
            EnvAction::Get   { key } => commands::env_cmd::get(key.as_deref())?,
            EnvAction::Set   { key, value } => {
                commands::env_cmd::set(&key, &value)?;
                // If sxhkd is running, reload it so wrapper scripts pick up the change
                let sxhkd_running = std::process::Command::new("pgrep")
                    .args(["-x", "sxhkd"]).output()
                    .map(|o| o.status.success()).unwrap_or(false);
                if sxhkd_running {
                    let _ = std::process::Command::new("pkill").args(["-USR1", "sxhkd"]).status();
                    println!("  {} sxhkd reloaded — change takes effect immediately", "✓".green());
                }
            }
            EnvAction::Unset { key } => commands::env_cmd::unset(&key)?,
            EnvAction::Init  => commands::env_cmd::init()?,
            EnvAction::Edit  => commands::env_cmd::edit()?,
        },

        Commands::Theme { wallpaper, reload, current } => {
            if current {
                commands::theme::current()?;
            } else {
                commands::log::append(&paths.state_dir, "theme", wallpaper.as_deref().unwrap_or("picker"));
                commands::theme::apply(wallpaper.as_deref(), reload)?;
            }
        }

        Commands::Rice { action } => match action {
            RiceAction::List    => commands::rice::list(&paths)?,
            RiceAction::Apply   { name, dry_run, force } => {
                commands::log::append(&paths.state_dir, "rice-apply", &name);
                commands::rice::apply(&paths, &name, dry_run, force)?;
            }
            RiceAction::Create  { name } => commands::rice::create(&paths, &name)?,
            RiceAction::Current => commands::rice::current(&paths)?,
        },

        Commands::SaveConfig => commands::config_backup::save_config(&paths, "manual", cli.json)?,
        Commands::RestoreConfig { backup } => commands::config_backup::restore_config(&paths, backup, cli.json)?,

        Commands::Hooks { action } => match action {
            HooksAction::List => commands::hooks::list(&paths, cli.json)?,
            HooksAction::Reset { module, pre } => commands::hooks::reset(&paths, &module, pre)?,
            HooksAction::Skip  { module, pre } => commands::hooks::skip(&paths, &module, pre)?,
            HooksAction::Run   { module, pre } => commands::hooks::run_hook(&paths, &module, pre)?,
        },

        Commands::Repo { action } => match action {
            RepoAction::Init   => commands::repo::init(&paths)?,
            RepoAction::Clone  => commands::repo::clone_repo(&paths)?,
            RepoAction::Push   => commands::repo::push(&paths)?,
            RepoAction::Pull   => commands::repo::pull(&paths)?,
            RepoAction::Status => commands::repo::status(&paths)?,
        },

        Commands::SelfUpdate => commands::selfupdate::run()?,
        Commands::Search => commands::search::run(&paths)?,
        Commands::Edit => commands::edit::run(&paths)?,
        Commands::Completions { shell } => commands::completions::run(&shell)?,
    }

    Ok(())
}

// Import colored for the check_root message
use colored::*;
