use anyhow::Result;
use colored::*;

pub fn run() -> Result<()> {
    println!("{}", "Updating vcli from source...".blue());

    // Find where vcli binary is
    let vcli_path = which::which("vcli")
        .unwrap_or_else(|_| std::path::PathBuf::from("/usr/local/bin/vcli"));

    // Try to find source directory (looks for Cargo.toml upward from binary)
    // Fallback: re-clone and rebuild
    let src_dir = std::env::var("VCLI_SRC_DIR").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        format!("{}/vcli", home)
    });

    let src_path = std::path::Path::new(&src_dir);

    if !src_path.exists() {
        println!("{}", format!("Source directory not found: {}", src_dir).yellow());
        println!("Set VCLI_SRC_DIR or clone the repo manually:");
        println!("  git clone <vcli-repo-url> ~/vcli");
        println!("  cd ~/vcli && cargo build --release");
        println!("  sudo cp target/release/vcli /usr/local/bin/vcli");
        return Ok(());
    }

    // git pull
    println!("{}", "Pulling latest changes...".blue());
    let pull_status = std::process::Command::new("git")
        .args(["-C", &src_dir, "pull"])
        .status()?;

    if !pull_status.success() {
        anyhow::bail!("git pull failed");
    }

    // cargo build --release
    println!("{}", "Building...".blue());
    let build_status = std::process::Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(&src_dir)
        .status()?;

    if !build_status.success() {
        anyhow::bail!("cargo build failed");
    }

    // Copy binary
    let new_binary = src_path.join("target/release/vcli");
    println!(
        "{} Installing to {}...",
        "→".blue(),
        vcli_path.display()
    );

    let copy_status = std::process::Command::new("sudo")
        .args(["cp", new_binary.to_str().unwrap(), vcli_path.to_str().unwrap()])
        .status()?;

    if !copy_status.success() {
        anyhow::bail!("Failed to copy binary (try running with sudo)");
    }

    println!("{}", "✓ vcli updated successfully!".green());
    Ok(())
}
