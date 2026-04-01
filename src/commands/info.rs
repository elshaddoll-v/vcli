//! vcli info <pkg> — display clean, human-readable package info.
use anyhow::Result;
use colored::*;

pub fn run(package: &str) -> Result<()> {
    // Get all properties in one call
    let output = std::process::Command::new("xbps-query")
        .args(["-RS", package])
        .output()?;

    if !output.status.success() || output.stdout.is_empty() {
        // Try local query
        let local = std::process::Command::new("xbps-query")
            .args(["-S", package])
            .output()?;

        if !local.status.success() || local.stdout.is_empty() {
            println!("{} Package '{}' not found.", "✗".red(), package);
            println!("  Search with: {}", format!("xbps-query -Rs {}", package).cyan());
            return Ok(());
        }

        print_info(&String::from_utf8_lossy(&local.stdout), true);
        return Ok(());
    }

    print_info(&String::from_utf8_lossy(&output.stdout), false);
    Ok(())
}

fn print_info(raw: &str, installed: bool) {
    // Parse key: value lines from xbps-query output
    let mut fields: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    for line in raw.lines() {
        if let Some(pos) = line.find(':') {
            let key = line[..pos].trim().to_lowercase().replace('-', "_");
            let val = line[pos + 1..].trim().to_string();
            if !val.is_empty() {
                fields.insert(key, val);
            }
        }
    }

    let name = fields.get("pkgname").cloned().unwrap_or_default();
    let version = fields.get("pkgver").cloned().unwrap_or_default();
    let desc = fields.get("short_desc").cloned().unwrap_or_default();
    let homepage = fields.get("homepage").cloned().unwrap_or_default();
    let license = fields.get("license").cloned().unwrap_or_default();
    let size = fields.get("installed_size").cloned().unwrap_or_default();
    let repo = fields.get("repository").cloned().unwrap_or_default();
    let maintainer = fields.get("maintainer").cloned().unwrap_or_default();
    let install_date = fields.get("install_date").cloned().unwrap_or_default();

    let status = if installed {
        format!(" {}", "[installed]".green())
    } else {
        String::new()
    };

    println!();
    println!("{}{}", name.bold().cyan(), status);
    println!("{}", "─".repeat(50).dimmed());
    println!("{}", desc);
    println!();

    let print_field = |label: &str, value: &str| {
        if !value.is_empty() {
            println!("  {:<14} {}", format!("{}:", label).bold(), value);
        }
    };

    print_field("Version", &version);
    print_field("Size", &size);
    print_field("License", &license);
    print_field("Repository", &repo);
    print_field("Maintainer", &maintainer);
    if !install_date.is_empty() {
        print_field("Installed", &install_date);
    }
    if !homepage.is_empty() {
        println!("  {:<14} {}", "Homepage:".bold(), homepage.cyan());
    }

    println!();
}
