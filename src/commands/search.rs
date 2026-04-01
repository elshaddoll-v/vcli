use anyhow::Result;
use colored::*;

use crate::config::ConfigPaths;

pub fn run(paths: &ConfigPaths) -> Result<()> {
    // Check fzf available
    if which::which("fzf").is_err() {
        println!("{}", "fzf is not installed.".yellow());
        println!("Install it with: xbps-install -S fzf");
        return Ok(());
    }

    println!("{}", "Searching packages with fzf...".blue());
    println!("(Type to filter, TAB to select multiple, ENTER to install)");
    println!();

    // Use xbps-query -Rs '' to get all available packages, pipe to fzf
    // xbps-query -Rs output: "[*] pkgname-ver description"
    // We extract just the name
    let xbps_output = std::process::Command::new("xbps-query")
        .args(["-Rs", ""])
        .output();

    let package_list = match xbps_output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // Parse lines: "[-] pkgname-version  description"
            // or "[*] pkgname-version  description"
            stdout
                .lines()
                .filter_map(|line| {
                    let line = line.trim();
                    // Strip the status prefix [*] or [-]
                    let rest = if line.starts_with("[*]") || line.starts_with("[-]") {
                        line[3..].trim()
                    } else {
                        line
                    };
                    // Get the pkgver (first word)
                    let pkgver = rest.split_whitespace().next()?;
                    let desc = rest[pkgver.len()..].trim();
                    // Strip version from pkgver
                    let (name, _) = crate::package::split_xbps_pkgver(pkgver);
                    Some(format!("{}\t{}", name, desc))
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        Err(_) => {
            anyhow::bail!("Failed to query xbps package database");
        }
    };

    // Pipe to fzf
    use std::io::Write;
    let mut fzf = std::process::Command::new("fzf")
        .args([
            "--multi",
            "--preview",
            "xbps-query -RS {1}",
            "--preview-window",
            "right:50%",
            "--prompt",
            "Install package> ",
            "--delimiter",
            "\t",
            "--with-nth",
            "1,2",
            "--bind",
            "ctrl-a:select-all",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    if let Some(stdin) = fzf.stdin.take() {
        let mut stdin = stdin;
        let _ = stdin.write_all(package_list.as_bytes());
    }

    let output = fzf.wait_with_output()?;

    if output.stdout.is_empty() {
        println!("{}", "No packages selected.".yellow());
        return Ok(());
    }

    let selected = String::from_utf8_lossy(&output.stdout);
    let packages: Vec<String> = selected
        .lines()
        .filter_map(|line| {
            let name = line.split('\t').next()?.trim().to_string();
            if name.is_empty() { None } else { Some(name) }
        })
        .collect();

    if packages.is_empty() {
        return Ok(());
    }

    println!();
    println!("{}", format!("Installing {} package(s):", packages.len()).blue());
    for pkg in &packages {
        println!("  {} {}", "+".green(), pkg);
    }

    // Install each selected package
    for pkg in &packages {
        crate::commands::simple::install(pkg, paths)?;
    }

    Ok(())
}
