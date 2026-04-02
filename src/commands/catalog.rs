//! Built-in module catalog — embedded directly in the binary.
//! No external files needed. `vcli init` and `vcli module list --catalog`
//! always have access to these regardless of install location.

use anyhow::Result;
use colored::*;

/// A built-in module definition
pub struct CatalogModule {
    pub name: &'static str,
    pub description: &'static str,
    pub content: &'static str,
}

/// All built-in modules — embedded at compile time
pub static CATALOG: &[CatalogModule] = &[
    // ── X11 / Wayland base ───────────────────────────────────────────────
    CatalogModule {
        name: "x11",
        description: "X11 display server (required for all X11 WMs and DEs)",
        content: "description: X11 display server base\n\npackages:\n  - xorg-minimal\n  - xorg-input-drivers\n  - xorg-video-drivers\n  - xinit\n  - xauth\n  - xrdb\n  - xset\n  - setxkbmap\n  - xclip\n  - xsel\n\nservices:\n  enabled:\n    - dbus\n",
    },
    CatalogModule {
        name: "wayland",
        description: "Wayland display server base",
        content: "description: Wayland display server base\n\npackages:\n  - wayland\n  - wayland-protocols\n  - xwayland\n  - xdg-desktop-portal\n  - xdg-utils\n  - wl-clipboard\n",
    },

    // ── Window Managers ──────────────────────────────────────────────────
    CatalogModule {
        name: "wm-i3",
        description: "i3 tiling window manager",
        content: "description: i3 tiling window manager\n\npackages:\n  - i3\n  - i3status\n  - i3lock\n  - dmenu\n  - xdotool\n  - wmctrl\n",
    },
    CatalogModule {
        name: "wm-bspwm",
        description: "bspwm tiling window manager",
        content: "description: bspwm tiling window manager\n\npackages:\n  - bspwm\n  - sxhkd\n  - xdo\n  - xdotool\n  - wmctrl\n",
    },
    CatalogModule {
        name: "wm-awesome",
        description: "awesome window manager",
        content: "description: awesome window manager\n\npackages:\n  - awesome\n  - vicious\n  - lua53-lpeg\n  - lua53-luafilesystem\n",
    },
    CatalogModule {
        name: "wm-openbox",
        description: "Openbox stacking window manager",
        content: "description: Openbox stacking window manager\n\npackages:\n  - openbox\n  - obconf\n",
    },
    CatalogModule {
        name: "wm-hyprland",
        description: "Hyprland Wayland compositor",
        content: "description: Hyprland Wayland compositor\n\npackages:\n  - hyprland\n  - xdg-desktop-portal-hyprland\n  - waybar\n  - wofi\n  - dunst\n  - hyprpaper\n  - grim\n  - slurp\n  - wl-clipboard\n",
    },
    CatalogModule {
        name: "wm-sway",
        description: "Sway Wayland tiling WM (i3 compatible)",
        content: "description: Sway Wayland tiling WM\n\npackages:\n  - sway\n  - swaylock\n  - swayidle\n  - waybar\n  - wofi\n  - mako\n  - grim\n  - slurp\n  - wl-clipboard\n",
    },
    CatalogModule {
        name: "wm-dwm",
        description: "dwm - suckless window manager (build deps)",
        content: "description: dwm build dependencies\n\npackages:\n  - libX11-devel\n  - libXft-devel\n  - libXinerama-devel\n  - base-devel\n  - git\n",
    },

    // ── Desktop Environments ─────────────────────────────────────────────
    CatalogModule {
        name: "de-xfce",
        description: "XFCE desktop environment",
        content: "description: XFCE desktop environment\n\npackages:\n  - xfce4\n  - xfce4-plugins\n  - xfce4-pulseaudio-plugin\n  - xfce4-whiskermenu-plugin\n  - thunar\n  - thunar-volman\n  - xfce4-terminal\n",
    },
    CatalogModule {
        name: "de-kde",
        description: "KDE Plasma desktop environment",
        content: "description: KDE Plasma desktop environment\n\npackages:\n  - kde5\n  - kde5-baseapps\n  - plasma-desktop\n  - plasma-nm\n  - konsole\n  - dolphin\n  - sddm\n",
    },
    CatalogModule {
        name: "de-gnome",
        description: "GNOME desktop environment",
        content: "description: GNOME desktop environment\n\npackages:\n  - gnome\n  - gnome-terminal\n  - gnome-tweaks\n  - gdm\n",
    },
    CatalogModule {
        name: "de-lxqt",
        description: "LXQt desktop environment",
        content: "description: LXQt desktop environment\n\npackages:\n  - lxqt\n  - openbox\n  - sddm\n",
    },

    // ── Shells ────────────────────────────────────────────────────────────
    CatalogModule {
        name: "shell-fish",
        description: "Fish shell with starship prompt and zoxide",
        content: "description: Fish shell with starship and zoxide\n\npackages:\n  - fish-shell\n  - starship\n  - zoxide\n  - atuin\n",
    },
    CatalogModule {
        name: "shell-zsh",
        description: "Zsh with plugins and starship prompt",
        content: "description: Zsh with plugins and starship\n\npackages:\n  - zsh\n  - zsh-autosuggestions\n  - zsh-syntax-highlighting\n  - zsh-history-substring-search\n  - starship\n  - zoxide\n  - atuin\n",
    },
    CatalogModule {
        name: "shell-bash",
        description: "Bash with completion and starship prompt",
        content: "description: Bash with completion and starship\n\npackages:\n  - bash\n  - bash-completion\n  - starship\n  - zoxide\n",
    },

    // ── CLI Tools ─────────────────────────────────────────────────────────
    CatalogModule {
        name: "cli-modern",
        description: "Modern CLI replacements (bat, eza, fd, ripgrep, etc.)",
        content: "description: Modern CLI tools\n\npackages:\n  - bat\n  - eza\n  - fd\n  - ripgrep\n  - delta\n  - bottom\n  - procs\n  - jq\n  - yq\n  - fzf\n  - zoxide\n  - tmux\n  - yazi\n  - atuin\n",
    },
    CatalogModule {
        name: "cli-dev",
        description: "Development tools (git, editors, build tools)",
        content: "description: Development tools\n\npackages:\n  - git\n  - base-devel\n  - gcc\n  - make\n  - cmake\n  - nodejs\n  - python3\n  - python3-pip\n  - neovim\n  - vim\n  - just\n  - lazygit\n  - tree\n",
    },
    CatalogModule {
        name: "cli-system",
        description: "System monitoring and management tools",
        content: "description: System tools\n\npackages:\n  - htop\n  - btop\n  - ncdu\n  - curl\n  - wget\n  - rsync\n  - unzip\n  - p7zip\n  - bc\n  - cronie\n\nservices:\n  enabled:\n    - cronie\n",
    },

    // ── Audio ─────────────────────────────────────────────────────────────
    CatalogModule {
        name: "audio-pipewire",
        description: "PipeWire audio (modern, recommended)",
        content: "description: PipeWire audio\n\npackages:\n  - pipewire\n  - pipewire-pulse\n  - pipewire-alsa\n  - alsa-pipewire\n  - wireplumber\n  - pamixer\n  - playerctl\n  - pavucontrol\n",
    },
    CatalogModule {
        name: "audio-pulseaudio",
        description: "PulseAudio (legacy)",
        content: "description: PulseAudio\n\npackages:\n  - pulseaudio\n  - pulseaudio-utils\n  - pamixer\n  - playerctl\n  - pavucontrol\n",
    },

    // ── Fonts ─────────────────────────────────────────────────────────────
    CatalogModule {
        name: "fonts-nerd",
        description: "Nerd Fonts (patched with icons)",
        content: "description: Nerd Fonts\n\npackages:\n  - nerd-fonts\n  - font-awesome5\n",
    },
    CatalogModule {
        name: "fonts-common",
        description: "Common system fonts (noto, ubuntu, dejavu)",
        content: "description: Common fonts\n\npackages:\n  - noto-fonts-ttf\n  - noto-fonts-emoji\n  - ttf-ubuntu-font-family\n  - dejavu-fonts-ttf\n  - terminus-font\n  - font-inconsolata-otf\n",
    },

    // ── Theming ───────────────────────────────────────────────────────────
    CatalogModule {
        name: "theming",
        description: "Theming tools (pywal, feh, lxappearance, icons)",
        content: "description: Theming tools\n\npackages:\n  - feh\n  - python3-pywal\n  - papirus-icon-theme\n  - lxappearance\n  - qt5ct\n  - gnome-themes-standard\n",
    },

    // ── Display Managers ──────────────────────────────────────────────────
    CatalogModule {
        name: "dm-lightdm",
        description: "LightDM display manager",
        content: "description: LightDM display manager\n\npackages:\n  - lightdm\n  - lightdm-gtk-greeter\n\nservices:\n  enabled:\n    - lightdm\n",
    },
    CatalogModule {
        name: "dm-sddm",
        description: "SDDM display manager (for KDE/LXQt)",
        content: "description: SDDM display manager\n\npackages:\n  - sddm\n\nservices:\n  enabled:\n    - sddm\n",
    },

    // ── Desktop Tools ─────────────────────────────────────────────────────
    CatalogModule {
        name: "desktop-tools",
        description: "Common desktop tools (rofi, dunst, picom, flameshot)",
        content: "description: Common desktop tools\n\npackages:\n  - dunst\n  - libnotify\n  - rofi\n  - flameshot\n  - maim\n  - picom\n  - redshift\n  - network-manager-applet\n  - brightnessctl\n  - udiskie\n  - udisks2\n  - polkit\n  - lxsession\n  - sxhkd\n  - wmctrl\n  - xdotool\n",
    },

    // ── Gaming ────────────────────────────────────────────────────────────
    CatalogModule {
        name: "gaming",
        description: "Gaming (steam, lutris, wine, gamemode)",
        content: "description: Gaming packages\n\npackages:\n  - steam\n  - lutris\n  - wine\n  - winetricks\n  - gamemode\n  - mangohud\n  - vulkan-loader\n",
    },

    // ── Media ─────────────────────────────────────────────────────────────
    CatalogModule {
        name: "media",
        description: "Media playback (mpv, vlc, mpd, ffmpeg)",
        content: "description: Media tools\n\npackages:\n  - mpv\n  - vlc\n  - ffmpeg\n  - mpd\n  - ncmpcpp\n  - mpc\n  - playerctl\n",
    },
];

/// List all catalog modules
pub fn list_catalog(json: bool) -> Result<()> {
    if json {
        let items: Vec<serde_json::Value> = CATALOG.iter().map(|m| {
            serde_json::json!({ "name": m.name, "description": m.description })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&items)?);
        return Ok(());
    }

    println!("{}", "Built-in module catalog:".blue().bold());
    println!("{}", "(These are always available — install with: vcli catalog install <name>)".dimmed());
    println!();

    let categories = [
        ("Display", &["x11", "wayland"]),
        ("Window Managers", &["wm-i3", "wm-bspwm", "wm-awesome", "wm-openbox", "wm-hyprland", "wm-sway", "wm-dwm"]),
        ("Desktop Environments", &["de-xfce", "de-kde", "de-gnome", "de-lxqt"]),
        ("Shells", &["shell-fish", "shell-zsh", "shell-bash"]),
        ("CLI Tools", &["cli-modern", "cli-dev", "cli-system"]),
        ("Audio", &["audio-pipewire", "audio-pulseaudio"]),
        ("Fonts", &["fonts-nerd", "fonts-common"]),
        ("Theming", &["theming"]),
        ("Display Managers", &["dm-lightdm", "dm-sddm"]),
        ("Desktop Tools", &["desktop-tools"]),
        ("Other", &["gaming", "media"]),
    ];

    for (category, names) in &categories {
        println!("  {}:", category.bold());
        for name in *names {
            if let Some(m) = CATALOG.iter().find(|m| m.name == *name) {
                println!("    {} {}  —  {}", "·".dimmed(), name.cyan(), m.description);
            }
        }
        println!();
    }

    println!("Install a module:  {}", "vcli catalog install <name>".cyan());
    println!("Enable it:         {}", "vcli module enable <name>".cyan());
    println!("Sync packages:     {}", "vcli sync".cyan());

    Ok(())
}

/// Install a catalog module into the user's void-config modules directory
pub fn install_catalog_module(paths: &crate::config::ConfigPaths, name: &str, enable: bool) -> Result<()> {
    let module = CATALOG.iter().find(|m| m.name == name)
        .ok_or_else(|| anyhow::anyhow!(
            "Module '{}' not found in catalog.\nRun 'vcli catalog list' to see all available modules.",
            name
        ))?;

    let modules_dir = paths.modules_dir();
    std::fs::create_dir_all(&modules_dir)?;

    let dest = modules_dir.join(format!("{}.yaml", name));
    if dest.exists() {
        println!("{} Module '{}' already exists at {:?}", "!".yellow(), name, dest);
        println!("  It will not be overwritten. Delete it first if you want to reset.");
    } else {
        std::fs::write(&dest, module.content)?;
        println!("{} Installed module '{}' to {:?}", "✓".green(), name, dest);
    }

    if enable {
        crate::commands::module::enable(paths, name, false)?;
    }

    Ok(())
}

/// Install all catalog modules at once
pub fn install_all(paths: &crate::config::ConfigPaths) -> Result<()> {
    let modules_dir = paths.modules_dir();
    std::fs::create_dir_all(&modules_dir)?;

    let mut installed = 0;
    let mut skipped = 0;

    for module in CATALOG {
        let dest = modules_dir.join(format!("{}.yaml", module.name));
        if dest.exists() {
            skipped += 1;
        } else {
            std::fs::write(&dest, module.content)?;
            installed += 1;
        }
    }

    println!("{} {} module(s) installed, {} skipped (already exist)",
        "✓".green(), installed, skipped);
    Ok(())
}
