# vcli

**A declarative package management CLI tool for Void Linux**, inspired by [dcli](https://gitlab.com/theblackdon/dcli) and NixOS. Define your entire system in YAML files, organize packages into reusable modules, and sync your setup across machines.

> A full port of dcli — same architecture, same config format, same commands — but for Void Linux using `xbps` and `runit` instead of `pacman`/`systemd`.

Built with Rust. Zero runtime dependencies.

---

## Installation

```bash
git clone <this-repo> ~/vcli
cd ~/vcli
./install.sh
```

**Prerequisites:**
- Void Linux
- Rust toolchain (installer handles this)

**Optional:**
- `fzf` — for interactive TUI features (`xbps-install -S fzf`)
- `flatpak` — for flatpak support
- `git` — for config sync across machines

---

## Quick Start

```bash
# 1. Initialize config
vcli init

# 2. Edit your host config
vcli edit

# 3. Sync system to match config
vcli sync

# 4. Preview changes before applying
vcli sync --dry-run
```

---

## Config Structure

```
~/.config/void-config/
├── config.yaml           # Pointer to active host
├── hosts/
│   └── {hostname}.yaml   # Your full configuration
├── modules/
│   ├── base.yaml         # Base packages
│   ├── gaming.yaml       # Example module
│   └── desktop/          # Directory module
│       ├── module.yaml
│       ├── packages.yaml
│       └── dotfiles/
└── state/                # Auto-managed state
```

### Host File (`hosts/{hostname}.yaml`)

```yaml
host: myvoidbox
description: My Void Linux desktop

enabled_modules:
  - gaming
  - development

packages:
  - firefox
  - alacritty

exclude:
  - nano

# Runit services (Void Linux uses runit, not systemd)
services:
  enabled:
    - dbus
    - NetworkManager
    - bluetoothd
  disabled:
    - sshd

flatpak_scope: user        # user or system
auto_prune: false          # Remove undeclared packages on sync
module_processing: parallel # parallel (default) or sequential

config_backups:
  enabled: true
  max_backups: 5
```

### Module File (`modules/gaming.yaml`)

```yaml
description: Gaming packages

packages:
  - steam
  - lutris
  - wine
  - gamemode
  - flatpak:com.valvesoftware.Steam

post_install_hook: scripts/setup-gaming.sh
hook_behavior: ask    # ask | always | once | skip
```

### Directory Module (`modules/desktop/`)

```
modules/desktop/
├── module.yaml          # Manifest
├── packages.yaml        # Package list
└── dotfiles/            # Auto-symlinked to ~/.config/
    ├── hypr/
    └── waybar/
```

`module.yaml`:
```yaml
description: Desktop environment
dotfiles_sync: true
post_install_hook: scripts/setup-desktop.sh
hook_behavior: once
```

---

## Core Commands

### Package Management

```bash
vcli install <pkg>         # Install and add to config
vcli remove <pkg>          # Remove and drop from config
vcli sync                  # Sync system to match config
vcli sync --dry-run        # Preview changes
vcli sync --prune          # Also remove undeclared packages
vcli sync --force          # Skip confirmation
vcli update                # xbps-install -Su (full update)
vcli merge                 # Capture installed packages into config
vcli merge --services      # Capture enabled runit services
vcli forget <pkg>          # Stop tracking (keep installed)
vcli find <pkg>            # Find where package is declared
```

### Modules

```bash
vcli module list                # List all modules
vcli module enable gaming       # Enable a module
vcli module enable              # Interactive selection (fzf)
vcli module disable gaming      # Disable a module
vcli module create mymodule     # Create new module from template
```

### Status & Validation

```bash
vcli status                     # Show config and sync status
vcli validate                   # Check config integrity
vcli validate --check-packages  # Also check xbps repo availability
```

### Config Backup & Restore

```bash
vcli save-config                # Manual config backup
vcli restore-config             # Interactive restore (fzf)
```

### Post-Install Hooks

```bash
vcli hooks list                 # Show all hooks and status
vcli hooks reset gaming         # Reset hook to run again
vcli hooks skip gaming          # Permanently skip hook
vcli hooks run gaming           # Manually run a hook
```

### Git Integration

```bash
vcli repo init                  # Set up git for void-config
vcli repo clone                 # Clone existing void-config
vcli repo push                  # Commit and push changes
vcli repo pull                  # Pull updates from remote
vcli repo status                # Show git status
```

### Other

```bash
vcli search                     # Interactive package search (fzf + xbps-query)
vcli edit                       # Open config file in $EDITOR
vcli self-update                # Rebuild and update vcli itself
vcli --json <command>           # JSON output for scripting
```

---

## Differences from dcli (Arch → Void)

| Feature | dcli (Arch) | vcli (Void) |
|---------|-------------|-------------|
| Package manager | `pacman` / AUR (`paru`/`yay`) | `xbps-install` / `xbps-remove` |
| Package query | `pacman -Qm` | `xbps-query -m` |
| Service manager | `systemctl` | `sv` / runit symlinks |
| Service enable | `systemctl enable --now` | `ln -s /etc/sv/X /var/service/` |
| Service disable | `systemctl disable --now` | `rm /var/service/X` |
| Config dir | `~/.config/arch-config/` | `~/.config/void-config/` |
| Env var override | `ARCH_CONFIG_DIR` | `VOID_CONFIG_DIR` |
| AUR support | Yes (paru/yay) | No AUR (xbps has void-packages) |
| Flatpak support | Yes | Yes |
| Lua scripting | Yes | Not yet (YAML only) |
| TUI (ratatui) | Yes | Not yet |

---

## Runit Services

Void Linux uses **runit** instead of systemd. vcli handles services by:

- **Enable**: creates a symlink `ln -sf /etc/sv/<name> /var/service/`
- **Disable**: removes the symlink `rm -f /var/service/<name>`
- Runit auto-starts enabled services

Service names must match directories in `/etc/sv/`. Common ones:
```yaml
services:
  enabled:
    - dbus
    - NetworkManager
    - bluetoothd
    - sshd
    - cupsd
    - cronie
    - docker
```

---

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `VOID_CONFIG_DIR` | `~/.config/void-config` | Config directory |
| `EDITOR` | `vi` | Editor for `vcli edit` |
| `VCLI_SRC_DIR` | `~/vcli` | Source dir for `vcli self-update` |

---

## License

0BSD — same as dcli.

## Credits

This project is a Void Linux port of [dcli](https://gitlab.com/theblackdon/dcli) by Black Don.
