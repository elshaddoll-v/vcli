//! Dotfile symlinking — called ONLY by `vcli dots` and `vcli rice apply`.
//! `vcli sync` never calls this. Packages and dotfiles are fully separate.
use anyhow::Result;
// use colored::*;
use std::path::{Path, PathBuf};

/// Symlink a single source → target with conflict detection and backup.
pub fn link(source: &Path, target: &Path, force: bool, dry_run: bool) -> Result<LinkResult> {
    if !source.exists() {
        return Ok(LinkResult::SourceMissing);
    }

    // Check if already correctly linked
    if target.is_symlink() {
        if let Ok(current) = std::fs::read_link(target) {
            if current == source {
                return Ok(LinkResult::AlreadyLinked);
            }
        }
    }

    if dry_run {
        if target.exists() || target.is_symlink() {
            return Ok(LinkResult::WouldBackup);
        }
        return Ok(LinkResult::WouldLink);
    }

    // Create parent dirs
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Handle existing target
    if target.exists() || target.is_symlink() {
        if !force {
            return Ok(LinkResult::Conflict);
        }
        // Backup existing
        let backup = PathBuf::from(format!("{}.vcli-bak", target.display()));
        std::fs::rename(target, &backup)?;
    }

    // Remove stale symlink
    if target.is_symlink() {
        std::fs::remove_file(target)?;
    }

    #[cfg(unix)]
    std::os::unix::fs::symlink(source, target)?;

    Ok(LinkResult::Linked)
}

/// Remove a symlink created by vcli, restoring backup if it exists.
pub fn unlink(target: &Path) -> Result<UnlinkResult> {
    if !target.is_symlink() {
        return Ok(UnlinkResult::NotASymlink);
    }

    std::fs::remove_file(target)?;

    // Restore backup if exists
    let backup = PathBuf::from(format!("{}.vcli-bak", target.display()));
    if backup.exists() {
        std::fs::rename(&backup, target)?;
        return Ok(UnlinkResult::Restored);
    }

    Ok(UnlinkResult::Removed)
}

pub enum LinkResult {
    Linked,
    AlreadyLinked,
    WouldLink,
    WouldBackup,
    Conflict,       // target exists, --force not set
    SourceMissing,
}

pub enum UnlinkResult {
    Removed,
    Restored,
    NotASymlink,
}

pub fn expand_tilde(path: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    if path == "~" {
        home
    } else if let Some(rest) = path.strip_prefix("~/") {
        format!("{}/{}", home, rest)
    } else {
        path.to_string()
    }
}
