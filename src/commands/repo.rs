use anyhow::Result;
use colored::*;
use std::io::{self, Write};
use std::process::Command;

use crate::config::ConfigPaths;

fn git(paths: &ConfigPaths, args: &[&str]) -> Result<std::process::ExitStatus> {
    Ok(Command::new("git")
        .args(["-C", paths.config_dir.to_str().unwrap()])
        .args(args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?)
}

pub fn init(paths: &ConfigPaths) -> Result<()> {
    let git_dir = paths.config_dir.join(".git");
    if git_dir.exists() {
        println!("{}", "Git repository already initialized.".yellow());
        return Ok(());
    }

    println!("{}", "Initializing git repository for void-config...".blue());
    git(paths, &["init"])?;
    git(paths, &["add", "."])?;
    git(paths, &["commit", "-m", "Initial vcli configuration"])?;

    println!();
    println!("{}", "✓ Git repository initialized".green());
    println!();
    println!("Add a remote and push:");
    println!("  git -C {} remote add origin <url>", paths.config_dir.display());
    println!("  vcli repo push");

    Ok(())
}

pub fn clone_repo(paths: &ConfigPaths) -> Result<()> {
    print!("Enter git repository URL to clone: ");
    io::stdout().flush()?;
    let mut url = String::new();
    io::stdin().read_line(&mut url)?;
    let url = url.trim();

    if url.is_empty() {
        anyhow::bail!("No URL provided");
    }

    println!("{}", format!("Cloning {}...", url).blue());
    let status = Command::new("git")
        .args(["clone", url, paths.config_dir.to_str().unwrap()])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if !status.success() {
        anyhow::bail!("git clone failed");
    }

    println!();
    println!("{}", "✓ Repository cloned. Run 'vcli sync' to apply.".green());
    Ok(())
}

pub fn push(paths: &ConfigPaths) -> Result<()> {
    println!("{}", "Committing and pushing void-config...".blue());

    git(paths, &["add", "."])?;

    let hostname = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string());

    let message = format!("vcli: sync from {}", hostname);
    let _ = git(paths, &["commit", "-m", &message]);

    let status = git(paths, &["push"])?;
    if !status.success() {
        anyhow::bail!("git push failed");
    }

    println!("{}", "✓ Pushed successfully".green());
    Ok(())
}

pub fn pull(paths: &ConfigPaths) -> Result<()> {
    println!("{}", "Pulling latest void-config...".blue());
    let status = git(paths, &["pull"])?;
    if !status.success() {
        anyhow::bail!("git pull failed");
    }
    println!("{}", "✓ Pulled. Run 'vcli sync' to apply changes.".green());
    Ok(())
}

pub fn status(paths: &ConfigPaths) -> Result<()> {
    let git_dir = paths.config_dir.join(".git");
    if !git_dir.exists() {
        println!("{}", "Not a git repository. Run 'vcli repo init' first.".yellow());
        return Ok(());
    }
    git(paths, &["status"])?;
    Ok(())
}
