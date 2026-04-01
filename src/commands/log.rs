//! vcli log — append-only operation log, show history of what vcli did.
use anyhow::Result;
use chrono::Utc;
use colored::*;
use std::io::Write;
use std::path::Path;

pub fn append(paths_state_dir: &Path, operation: &str, details: &str) {
    let log_file = paths_state_dir.join("vcli.log");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
    {
        let _ = writeln!(
            file,
            "{} | {} | {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            operation,
            details
        );
    }
}

pub fn run(paths_state_dir: &Path, lines: usize) -> Result<()> {
    let log_file = paths_state_dir.join("vcli.log");

    if !log_file.exists() {
        println!("{}", "No log entries yet.".dimmed());
        println!("vcli will record operations as you use it.");
        return Ok(());
    }

    let content = std::fs::read_to_string(&log_file)?;
    let all_lines: Vec<&str> = content.lines().collect();
    let total = all_lines.len();

    // Show last N lines
    let start = if total > lines { total - lines } else { 0 };
    let to_show = &all_lines[start..];

    println!(
        "{}",
        format!("vcli operation log (last {} of {} entries):", to_show.len(), total)
            .blue()
            .bold()
    );
    println!();

    for line in to_show {
        let parts: Vec<&str> = line.splitn(3, " | ").collect();
        if parts.len() == 3 {
            let timestamp = parts[0].dimmed();
            let op = parts[1];
            let detail = parts[2];

            // Color-code by operation type
            let op_colored = match op {
                "install" | "add"    => op.green().to_string(),
                "remove"             => op.red().to_string(),
                "sync"               => op.blue().to_string(),
                "update"             => op.cyan().to_string(),
                "module-enable"      => op.green().to_string(),
                "module-disable"     => op.yellow().to_string(),
                _                    => op.normal().to_string(),
            };

            println!("  {}  {:<16}  {}", timestamp, op_colored, detail);
        } else {
            println!("  {}", line.dimmed());
        }
    }

    println!();
    println!("Log file: {}", log_file.display().to_string().dimmed());

    Ok(())
}
