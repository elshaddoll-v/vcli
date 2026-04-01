//! vcli desktop — scaffold a complete desktop setup correctly.
//! Key fix: sxhkd uses wrapper scripts not raw $VARS, so env changes work instantly.
use anyhow::Result;
use colored::*;
use std::fs;
use std::path::Path;
use std::os::unix::fs::PermissionsExt;

use crate::config::ConfigPaths;

pub fn setup(wm: &str, paths: &ConfigPaths) -> Result<()> {
    let home = std::env::var("HOME").unwrap_or_default();

    println!("{}", format!("Setting up desktop for {} on Void Linux...", wm).blue().bold());
    println!();

    // 1. Environment file
    if !Path::new(&format!("{}/.config/environment.sh", home)).exists() {
        super::env_cmd::init()?;
        println!();
    }

    // 2. xprofile
    write_xprofile(&home)?;

    // 3. xinitrc
    write_xinitrc(&home, wm)?;

    // 4. Wrapper scripts FIRST (sxhkdrc depends on these)
    write_wrapper_scripts(&home)?;

    // 5. sxhkdrc (uses wrapper scripts, not raw $VARS)
    write_sxhkdrc(&home)?;

    // 6. Helper scripts (fzf-based productivity)
    write_helper_scripts(&home)?;

    // 7. Desktop module for vcli sync
    write_desktop_module(paths, wm)?;

    // 8. Setup fish function so vcli auto-elevates correctly
    write_fish_function(&home)?;

    println!();
    println!("{}", "✓ Desktop scaffold complete!".green().bold());
    println!();
    println!("{}", "Next steps:".bold());
    println!("  1. {} — install all desktop packages", "vcli sync".cyan());
    println!("  2. {} — start X", "startx".cyan());
    println!("  3. {} — change terminal everywhere instantly", "vcli env set TERMINAL foot".cyan());
    println!("  4. {} — apply a wallpaper + theme", "vcli theme ~/Pictures/wallpapers/wall.jpg".cyan());
    println!();
    println!("{}", "Install wallust for theme generation:".yellow());
    println!("  cargo install wallust");
    println!("  OR: vcli add python3-pywal  (pywal is in xbps, wallust is not)");

    Ok(())
}

fn write_xprofile(home: &str) -> Result<()> {
    let path = format!("{}/.xprofile", home);
    if Path::new(&path).exists() {
        println!("  {} ~/.xprofile exists — skipping", "·".dimmed());
        return Ok(());
    }

    // xprofile sources environment.sh so all wrappers pick it up at X start
    let content = r#"#!/bin/sh
# ~/.xprofile — sourced once at X startup by startx and display managers

# Load central environment variables
[ -f "$HOME/.config/environment.sh" ] && . "$HOME/.config/environment.sh"

# Keyboard: set your layout here
setxkbmap us

# Faster key repeat (essential for vim users)
xset r rate 300 50

# Disable screen blanking
xset s off
xset -dpms

# Mouse: disable acceleration
xset m 0 0

# Start sxhkd — reads sxhkdrc, uses wrapper scripts (not raw $VARS)
sxhkd &

# Notification daemon
dunst &

# Compositor (uncomment after installing picom)
# picom --daemon &

# Restore last wallpaper
[ -f "$HOME/.cache/vcli/current-wallpaper" ] && \
    feh --bg-scale "$(cat $HOME/.cache/vcli/current-wallpaper)" 2>/dev/null &

# Polybar / status bar (uncomment after configuring)
# polybar main &
"#;

    fs::write(&path, content)?;
    println!("  {} Created ~/.xprofile", "✓".green());
    Ok(())
}

fn write_xinitrc(home: &str, wm: &str) -> Result<()> {
    let path = format!("{}/.xinitrc", home);
    if Path::new(&path).exists() {
        println!("  {} ~/.xinitrc exists — skipping", "·".dimmed());
        return Ok(());
    }

    let wm_cmd = match wm {
        "dwm"     => "exec dwm",
        "awesome" => "exec awesome",
        "bspwm"   => "exec bspwm",
        "openbox" => "exec openbox-session",
        "xmonad"  => "exec xmonad",
        _         => "exec i3",   // i3 is default
    };

    let content = format!(r#"#!/bin/sh
# ~/.xinitrc — run with: startx

# Load environment + start background services
. "$HOME/.xprofile"

# Start window manager (last line, runs in foreground)
{}
"#, wm_cmd);

    fs::write(&path, &content)?;
    println!("  {} Created ~/.xinitrc (WM: {})", "✓".green(), wm.cyan());
    Ok(())
}

/// THE KEY FIX: wrapper scripts read environment.sh at runtime.
/// sxhkdrc calls `term`, `browser`, `editor` — not `$TERMINAL` directly.
/// When you run `vcli env set TERMINAL foot` + `pkill -USR1 sxhkd`,
/// the next keypress reads the updated file. No re-login needed.
fn write_wrapper_scripts(home: &str) -> Result<()> {
    let bin_dir = format!("{}/.local/bin", home);
    fs::create_dir_all(&bin_dir)?;

    let wrappers: &[(&str, &str)] = &[
        ("term", r#"#!/bin/sh
# Wrapper: reads TERMINAL from environment.sh at runtime
. "$HOME/.config/environment.sh" 2>/dev/null
exec ${TERMINAL:-alacritty} "$@"
"#),
        ("browser", r#"#!/bin/sh
. "$HOME/.config/environment.sh" 2>/dev/null
exec ${BROWSER:-firefox} "$@"
"#),
        ("editor", r#"#!/bin/sh
. "$HOME/.config/environment.sh" 2>/dev/null
exec ${EDITOR:-nvim} "$@"
"#),
        ("files", r#"#!/bin/sh
. "$HOME/.config/environment.sh" 2>/dev/null
exec ${FILE_MANAGER:-lf} "$@"
"#),
        ("launcher", r#"#!/bin/sh
. "$HOME/.config/environment.sh" 2>/dev/null
exec ${LAUNCHER:-rofi -show drun} "$@"
"#),
        ("locker", r#"#!/bin/sh
. "$HOME/.config/environment.sh" 2>/dev/null
exec ${LOCKER:-i3lock -c 000000} "$@"
"#),
    ];

    for (name, content) in wrappers {
        let path = format!("{}/{}", bin_dir, name);
        if !Path::new(&path).exists() {
            fs::write(&path, content)?;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
            println!("  {} Created ~/.local/bin/{}", "✓".green(), name);
        }
    }

    Ok(())
}

fn write_sxhkdrc(home: &str) -> Result<()> {
    let dir = format!("{}/.config/sxhkd", home);
    let path = format!("{}/sxhkdrc", dir);
    fs::create_dir_all(&dir)?;

    if Path::new(&path).exists() {
        println!("  {} ~/.config/sxhkd/sxhkdrc exists — skipping", "·".dimmed());
        return Ok(());
    }

    // Uses wrapper scripts (term, browser, editor, launcher, locker)
    // NOT raw $TERMINAL etc. — this is the fix.
    // Reload: pkill -USR1 sxhkd
    let content = r#"# ~/.config/sxhkd/sxhkdrc
# Universal keybinds — works across i3, dwm, bspwm, awesome
# Uses wrapper scripts in ~/.local/bin/ which read ~/.config/environment.sh
# To change terminal: vcli env set TERMINAL foot && pkill -USR1 sxhkd
# Reload this file: super + Escape

# ── Terminals ────────────────────────────────────────────────────────────────
super + Return
    term

super + shift + Return
    term --class floating

# Scratchpad terminal
super + grave
    term --class scratchpad

# ── Apps ─────────────────────────────────────────────────────────────────────
super + b
    browser

super + shift + e
    files

# ── Launcher ─────────────────────────────────────────────────────────────────
super + d
    launcher

super + shift + d
    rofi -show window -show-icons

# ── Editor ───────────────────────────────────────────────────────────────────
super + e
    term -e editor

# ── Productivity scripts ──────────────────────────────────────────────────────
# Fuzzy file finder + open in editor
super + f
    term -e sxhkd-open-file

# Jump to directory (zoxide + fzf)
super + z
    term -e sxhkd-cd-dir

# Search file contents (ripgrep + fzf)
super + shift + f
    term -e sxhkd-search-files

# Open project (finds git repos)
super + p
    term -e sxhkd-open-project

# Window switcher
super + Tab
    sxhkd-window-switch

# ── System keys ───────────────────────────────────────────────────────────────
# Volume (pamixer)
XF86AudioRaiseVolume
    pamixer -i 5 && notify-send -h int:value:$(pamixer --get-volume) -h string:synchronous:volume "Volume" "$(pamixer --get-volume)%"

XF86AudioLowerVolume
    pamixer -d 5 && notify-send -h int:value:$(pamixer --get-volume) -h string:synchronous:volume "Volume" "$(pamixer --get-volume)%"

XF86AudioMute
    pamixer -t && notify-send -h string:synchronous:volume "Volume" "$(pamixer --get-mute && echo Muted || echo Unmuted)"

XF86AudioMicMute
    pamixer --default-source -t

# Brightness (brightnessctl)
XF86MonBrightnessUp
    brightnessctl set +5% && notify-send -h string:synchronous:brightness "Brightness" "$(brightnessctl -m | cut -d, -f4)"

XF86MonBrightnessDown
    brightnessctl set 5%- && notify-send -h string:synchronous:brightness "Brightness" "$(brightnessctl -m | cut -d, -f4)"

# Screenshot
Print
    flameshot gui

shift + Print
    mkdir -p ~/Pictures/screenshots && flameshot full -p ~/Pictures/screenshots/

# Lock screen
super + shift + x
    locker

# ── Theming ────────────────────────────────────────────────────────────────────
# Wallpaper/theme picker (opens fzf)
super + shift + w
    vcli theme

# Reload sxhkd + show notification
super + Escape
    pkill -USR1 sxhkd && notify-send "sxhkd" "Config reloaded" -t 1500

# ── Close window ─────────────────────────────────────────────────────────────
super + q
    xdotool getactivewindow windowclose

# ── Session ──────────────────────────────────────────────────────────────────
super + shift + q
    rofi -show power-menu -modi "power-menu:rofi-power-menu" 2>/dev/null || \
    rofi -dmenu -p "Power" <<< $'lock\nlogout\nreboot\npoweroff' | \
    xargs -I{} sh -c 'case {} in lock) locker;; logout) pkill -u $USER;; reboot) reboot;; poweroff) poweroff;; esac'
"#;

    fs::write(&path, content)?;
    println!("  {} Created ~/.config/sxhkd/sxhkdrc", "✓".green());
    Ok(())
}

fn write_helper_scripts(home: &str) -> Result<()> {
    let bin_dir = format!("{}/.local/bin", home);

    let scripts: &[(&str, &str)] = &[
        ("sxhkd-open-file", r#"#!/bin/sh
# fzf file finder — opens in $EDITOR via wrapper
. "$HOME/.config/environment.sh" 2>/dev/null
file=$(fd --type f --hidden --follow --exclude .git --exclude node_modules \
          --exclude .cache 2>/dev/null \
    | fzf --preview 'bat --style=numbers --color=always --line-range=:100 {} 2>/dev/null || head -50 {}' \
          --preview-window=right:60% \
          --prompt="Open file> ")
[ -n "$file" ] && exec ${EDITOR:-nvim} "$file"
"#),
        ("sxhkd-cd-dir", r#"#!/bin/sh
# Jump to directory — zoxide if available, fd fallback
. "$HOME/.config/environment.sh" 2>/dev/null
if command -v zoxide >/dev/null 2>&1; then
    dir=$(zoxide query -l 2>/dev/null \
        | fzf --preview 'ls -la --color=always {}' \
              --preview-window=right:40% \
              --prompt="Jump to> ")
else
    dir=$(fd --type d --hidden --exclude .git --exclude node_modules \
             --exclude .cache 2>/dev/null \
        | fzf --preview 'ls -la --color=always {}' \
              --preview-window=right:40% \
              --prompt="Jump to> ")
fi
[ -n "$dir" ] && cd "$dir" && exec ${SHELL:-sh}
"#),
        ("sxhkd-search-files", r#"#!/bin/sh
# Search file contents — ripgrep + fzf, opens result in editor
. "$HOME/.config/environment.sh" 2>/dev/null
RG_PREFIX="rg --column --line-number --no-heading --color=always --smart-case"
result=$(: | fzf --disabled --ansi \
    --bind "start:reload:$RG_PREFIX ''" \
    --bind "change:reload:sleep 0.1; $RG_PREFIX {q} || true" \
    --delimiter : \
    --preview 'bat --style=numbers --color=always {1} --highlight-line {2} 2>/dev/null' \
    --preview-window 'right:60%:+{2}+3/3' \
    --prompt="Search> ")
[ -z "$result" ] && exit 0
file=$(echo "$result" | cut -d: -f1)
line=$(echo "$result" | cut -d: -f2)
exec ${EDITOR:-nvim} +"$line" "$file"
"#),
        ("sxhkd-open-project", r#"#!/bin/sh
# Open a git project directory
. "$HOME/.config/environment.sh" 2>/dev/null
search_dirs="$HOME/projects $HOME/dev $HOME/code $HOME/work $HOME"
dir=$(find $search_dirs -maxdepth 3 -name '.git' -type d 2>/dev/null \
    | sed 's|/.git$||' \
    | sort -u \
    | fzf --preview 'ls -la --color=always {}' \
          --preview-window=right:40% \
          --prompt="Open project> ")
[ -n "$dir" ] && cd "$dir" && exec ${TERMINAL:-alacritty}
"#),
        ("sxhkd-window-switch", r#"#!/bin/sh
# Switch windows via rofi
wmctrl -l 2>/dev/null \
    | awk '{$1=$2=$3=""; gsub(/^ +/,""); print NR". "$0}' \
    | rofi -dmenu -p "Switch to" -i \
    | grep -o '^[0-9]*' \
    | xargs -I{} sh -c \
        'line=$(wmctrl -l | sed -n "{}p"); id=$(echo "$line" | awk "{print \$1}"); wmctrl -ia "$id"'
"#),
    ];

    for (name, content) in scripts {
        let path = format!("{}/{}", bin_dir, name);
        if !Path::new(&path).exists() {
            fs::write(&path, content)?;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
            println!("  {} Created ~/.local/bin/{}", "✓".green(), name);
        }
    }

    // Create standard dirs
    fs::create_dir_all(format!("{}/Pictures/screenshots", home))?;
    fs::create_dir_all(format!("{}/Pictures/wallpapers", home))?;
    fs::create_dir_all(format!("{}/projects", home))?;
    println!("  {} Created ~/Pictures/screenshots, ~/Pictures/wallpapers, ~/projects", "✓".green());

    Ok(())
}

fn write_desktop_module(paths: &ConfigPaths, wm: &str) -> Result<()> {
    let module_path = paths.modules_dir().join("desktop.yaml");
    fs::create_dir_all(paths.modules_dir())?;

    if module_path.exists() {
        println!("  {} modules/desktop.yaml exists — skipping", "·".dimmed());
        return Ok(());
    }

    let wm_pkg = match wm {
        "dwm"     => "dwm",
        "awesome" => "awesome",
        "bspwm"   => "bspwm",
        "openbox" => "openbox",
        _         => "i3",
    };

    // Only packages that are actually in Void xbps repos
    let content = format!(r#"description: Desktop environment — {wm}

packages:
  # Window manager
  - {wm_pkg}

  # X server
  - xorg-minimal
  - xorg-input-drivers
  - xinit
  - xauth
  - xrandr
  - xset
  - xsetroot
  - xdotool
  - wmctrl
  - xclip

  # Keybind daemon
  - sxhkd

  # Launcher
  - rofi

  # Fuzzy finder + tools
  - fzf
  - fd
  - ripgrep
  - bat
  - eza

  # Notifications
  - dunst
  - libnotify

  # Wallpaper
  - feh

  # Audio
  - pamixer
  - pipewire
  - pipewire-pulse
  - alsa-utils

  # Brightness
  - brightnessctl

  # Screenshot
  - flameshot

  # Lock
  - i3lock

  # Fonts
  - nerd-fonts-ttf

  # Terminal
  - alacritty

  # Browser
  - firefox

  # Compositor
  - picom

  # Navigation
  - zoxide
  - starship

services:
  enabled:
    - dbus

post_install_hook: scripts/setup-desktop.sh
hook_behavior: once
"#, wm = wm, wm_pkg = wm_pkg);

    fs::write(&module_path, content)?;
    println!("  {} Created modules/desktop.yaml", "✓".green());

    // Post-install hook
    let scripts_dir = paths.config_dir.join("scripts");
    fs::create_dir_all(&scripts_dir)?;
    let hook = scripts_dir.join("setup-desktop.sh");

    let hook_content = r#"#!/bin/bash
# Post-install desktop setup hook — runs once after vcli sync

set -e

# Get the actual user (not root)
REAL_USER="${SUDO_USER:-$(logname 2>/dev/null || echo $USER)}"

echo "==> Setting up desktop for user: $REAL_USER"

# Enable dbus service (required for most desktop apps)
ln -sf /etc/sv/dbus /var/service/ 2>/dev/null && echo "  ✓ dbus enabled" || true

# Add user to all required groups
for group in audio video input bluetooth plugdev networkmanager; do
    if getent group "$group" >/dev/null 2>&1; then
        usermod -aG "$group" "$REAL_USER" 2>/dev/null && echo "  ✓ Added to $group" || true
    fi
done

# Make sure ~/.local/bin is in PATH for the user
if ! grep -q '.local/bin' /home/$REAL_USER/.profile 2>/dev/null; then
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> /home/$REAL_USER/.profile
    echo "  ✓ Added ~/.local/bin to PATH"
fi

# Create XDG dirs
sudo -u "$REAL_USER" mkdir -p \
    /home/$REAL_USER/.local/bin \
    /home/$REAL_USER/.config \
    /home/$REAL_USER/.cache/vcli \
    /home/$REAL_USER/Pictures/wallpapers \
    /home/$REAL_USER/Pictures/screenshots \
    /home/$REAL_USER/projects

echo ""
echo "==> Desktop setup complete!"
echo "    Log out and back in for group changes to take effect."
echo "    Then run: startx"
"#;

    fs::write(&hook, hook_content)?;
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755))?;
    println!("  {} Created scripts/setup-desktop.sh", "✓".green());

    Ok(())
}

/// Write the fish function that handles sudo correctly.
/// This replaces the broken auto-sudo in the binary.
fn write_fish_function(home: &str) -> Result<()> {
    let functions_dir = format!("{}/.config/fish/functions", home);
    fs::create_dir_all(&functions_dir)?;

    let path = format!("{}/vcli.fish", functions_dir);

    let content = r#"# ~/.config/fish/functions/vcli.fish
# Smart vcli wrapper:
# - Commands that need root automatically use sudo -E (preserves HOME)
# - Read-only commands run as your user
# - sudo -E ensures vcli finds your config, not root's config

function vcli
    set root_commands sync install add remove update doctor orphans

    if contains -- $argv[1] $root_commands
        # Check if already root
        if test (id -u) = "0"
            command vcli $argv
        else
            sudo -E /usr/local/bin/vcli $argv
        end
    else
        command vcli $argv
    end
end
"#;

    if Path::new(&path).exists() {
        println!("  {} ~/.config/fish/functions/vcli.fish exists — skipping", "·".dimmed());
        return Ok(());
    }

    fs::write(&path, content)?;
    println!("  {} Created ~/.config/fish/functions/vcli.fish", "✓".green());
    println!("    {} vcli now auto-elevates only when needed", "→".blue());

    Ok(())
}
