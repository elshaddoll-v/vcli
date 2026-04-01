//! vcli theme — apply wallpaper and regenerate terminal/rofi colors.
//! Supports wallust (cargo install wallust) and pywal (xbps: python3-pywal).
//! Falls back gracefully if neither is installed.
use anyhow::Result;
use colored::*;
use std::path::Path;

pub fn apply(wallpaper: Option<&str>, reload_only: bool) -> Result<()> {
    let home = std::env::var("HOME").unwrap_or_default();

    if reload_only {
        println!("{}", "Reloading theme components...".blue());
        return reload_all(&home);
    }

    let wallpaper = match wallpaper {
        Some(w) => w.to_string(),
        None => pick_wallpaper(&home)?,
    };

    // Expand tilde
    let wallpaper = if wallpaper.starts_with("~/") {
        format!("{}/{}", home, &wallpaper[2..])
    } else {
        wallpaper
    };

    if !Path::new(&wallpaper).exists() {
        anyhow::bail!("Wallpaper not found: {}\nPut images in ~/Pictures/wallpapers/", wallpaper);
    }

    println!("{}", format!("Applying theme from: {}", wallpaper).blue());
    println!();

    // Set wallpaper with feh first (instant visual feedback)
    if which::which("feh").is_ok() {
        std::process::Command::new("feh")
            .args(["--bg-scale", &wallpaper])
            .status()?;
        println!("  {} Wallpaper set", "✓".green());
    } else {
        println!("  {} feh not installed — install with: vcli add feh", "!".yellow());
    }

    // Save path for restore on next login
    let cache_dir = Path::new(&home).join(".cache/vcli");
    std::fs::create_dir_all(&cache_dir)?;
    std::fs::write(cache_dir.join("current-wallpaper"), &wallpaper)?;

    // Generate colors — try wallust first, then pywal, then skip
    let color_generator = detect_color_generator();
    match color_generator {
        ColorGenerator::Wallust => {
            println!("  {} Running wallust...", "→".blue());
            let status = std::process::Command::new("wallust")
                .args(["run", &wallpaper])
                .status()?;
            if status.success() {
                println!("  {} Colors generated with wallust", "✓".green());
            } else {
                println!("  {} wallust failed — colors not updated", "!".yellow());
            }
        }
        ColorGenerator::Pywal => {
            println!("  {} Running pywal...", "→".blue());
            let status = std::process::Command::new("wal")
                .args(["-i", &wallpaper, "--backend", "wal"])
                .status()?;
            if status.success() {
                println!("  {} Colors generated with pywal", "✓".green());
            } else {
                println!("  {} pywal failed — colors not updated", "!".yellow());
            }
        }
        ColorGenerator::None => {
            println!("  {} No color generator found", "!".yellow());
            println!("    Install wallust: {}", "cargo install wallust".cyan());
            println!("    OR pywal: {}", "vcli add python3-pywal".cyan());
            println!("  Wallpaper set, but terminal colors unchanged.");
        }
    }

    reload_all(&home)?;

    println!();
    println!("{}", "✓ Theme applied!".green().bold());
    Ok(())
}

pub fn reload_all(home: &str) -> Result<()> {
    let mut reloaded: Vec<&str> = Vec::new();

    // Reload dunst
    let dunst_running = std::process::Command::new("pgrep")
        .arg("-x").arg("dunst").output()
        .map(|o| o.status.success()).unwrap_or(false);
    if dunst_running {
        let _ = std::process::Command::new("pkill").args(["-USR2", "dunst"]).status();
        reloaded.push("dunst");
    }

    // Touch alacritty config so it hot-reloads
    let alacritty_cfg = Path::new(home).join(".config/alacritty/alacritty.toml");
    let alacritty_cfg2 = Path::new(home).join(".config/alacritty/alacritty.yml");
    if alacritty_cfg.exists() || alacritty_cfg2.exists() {
        let cfg = if alacritty_cfg.exists() { &alacritty_cfg } else { &alacritty_cfg2 };
        let _ = std::process::Command::new("touch").arg(cfg).status();
        reloaded.push("alacritty");
    }

    // Reload sxhkd (picks up any changes)
    let sxhkd_running = std::process::Command::new("pgrep")
        .arg("-x").arg("sxhkd").output()
        .map(|o| o.status.success()).unwrap_or(false);
    if sxhkd_running {
        let _ = std::process::Command::new("pkill").args(["-USR1", "sxhkd"]).status();
        reloaded.push("sxhkd");
    }

    // Source pywal colors for rofi if available
    let wal_colors = Path::new(home).join(".cache/wal/colors-rofi-dark.rasi");
    if wal_colors.exists() {
        reloaded.push("rofi colors");
    }

    if !reloaded.is_empty() {
        println!("  {} Reloaded: {}", "✓".green(), reloaded.join(", "));
    }

    Ok(())
}

enum ColorGenerator {
    Wallust,
    Pywal,
    None,
}

fn detect_color_generator() -> ColorGenerator {
    if which::which("wallust").is_ok() {
        return ColorGenerator::Wallust;
    }
    if which::which("wal").is_ok() {
        return ColorGenerator::Pywal;
    }
    ColorGenerator::None
}

fn pick_wallpaper(home: &str) -> Result<String> {
    if which::which("fzf").is_err() {
        anyhow::bail!(
            "No wallpaper specified and fzf not found.\nUsage: vcli theme /path/to/wallpaper.jpg"
        );
    }

    let wallpaper_dirs = [
        format!("{}/Pictures/wallpapers", home),
        format!("{}/Pictures", home),
        format!("{}/wallpapers", home),
    ];

    let existing_dir = wallpaper_dirs.iter()
        .find(|d| Path::new(d.as_str()).exists())
        .ok_or_else(|| anyhow::anyhow!(
            "No wallpaper directory found.\nCreate ~/Pictures/wallpapers/ and add some images."
        ))?;

    // Find images
    let find_output = std::process::Command::new("find")
        .args([existing_dir, "-type", "f",
               "-name", "*.jpg",   "-o",
               "-name", "*.jpeg",  "-o",
               "-name", "*.png",   "-o",
               "-name", "*.webp"])
        .output()?;

    let files = String::from_utf8_lossy(&find_output.stdout);
    if files.trim().is_empty() {
        anyhow::bail!(
            "No images found in {}.\nAdd some .jpg or .png files.", existing_dir
        );
    }

    // fzf picker with image preview if possible
    use std::io::Write;
    let preview_cmd = if which::which("kitty").is_ok() {
        "kitty icat --transfer-mode=memory --unicode-placeholder --stdin=no --place=60x30@0x0 {}"
    } else {
        "echo {}"
    };

    let mut fzf = std::process::Command::new("fzf")
        .args(["--prompt", "Select wallpaper> ",
               "--preview", preview_cmd,
               "--preview-window", "right:0"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    if let Some(stdin) = fzf.stdin.take() {
        let mut stdin = stdin;
        let _ = stdin.write_all(files.as_bytes());
    }

    let out = fzf.wait_with_output()?;
    let selected = String::from_utf8_lossy(&out.stdout).trim().to_string();

    if selected.is_empty() {
        anyhow::bail!("No wallpaper selected");
    }

    Ok(selected)
}

pub fn current() -> Result<()> {
    let home = std::env::var("HOME").unwrap_or_default();
    let cache_file = Path::new(&home).join(".cache/vcli/current-wallpaper");
    if cache_file.exists() {
        let path = std::fs::read_to_string(&cache_file)?;
        println!("Current wallpaper: {}", path.trim().cyan());
        let gen = detect_color_generator();
        println!("Color generator:   {}", match gen {
            ColorGenerator::Wallust => "wallust".green().to_string(),
            ColorGenerator::Pywal   => "pywal (wal)".green().to_string(),
            ColorGenerator::None    => "none installed".yellow().to_string(),
        });
    } else {
        println!("{}", "No wallpaper set via vcli yet.".yellow());
    }
    Ok(())
}
