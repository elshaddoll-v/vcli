# vcli

A declarative package manager for **Void Linux**, inspired by [dcli](https://gitlab.com/theblackdon/dcli) and NixOS.

Define your entire system in YAML. One command installs everything on a fresh machine.

```
vcli sync          # apply your config to the system
vcli install nvim  # install + track a package
vcli doctor        # full system health check
vcli theme         # apply wallpaper + generate colors
```

Built with Rust. Zero runtime dependencies. Works with any shell.

---

## Install on a fresh Void Linux

```sh
sudo xbps-install -Sy git curl

# Clone and build
git clone https://github.com/elshaddoll-v/vcli.git
cd vcli
chmod +x install.sh
./install.sh
```

The installer:
1. Installs Rust if not present
2. Builds vcli from source
3. Copies the binary to `/usr/local/bin/vcli`

**Requirements:** Void Linux (glibc or musl), git

**Optional:** `fzf` for interactive menus, `python3-pywal` for theming

---

## Quick Start

```sh
# Initialize config
vcli init

# See what's on your system
vcli status
vcli list

# Track all currently installed packages
vcli merge

# Sync system to match config
vcli sync
```

---

## Config Structure

```
~/.config/void-config/
├── config.yaml              # points to your host file
├── hosts/
│   └── mymachine.yaml       # your full system config
├── modules/
│   ├── base.yaml            # essential packages
│   ├── desktop.yaml         # X11, WM, apps
│   ├── development.yaml     # dev tools
│   └── gaming.yaml          # optional modules
└── state/                   # auto-managed, gitignored
```

### Host file (`hosts/mymachine.yaml`)

```yaml
host: mymachine
description: My Void Linux desktop

enabled_modules:
  - desktop
  - development

packages:
  - firefox
  - alacritty

services:
  enabled:
    - dbus
    - NetworkManager
  disabled:
    - sshd

flatpak_scope: user
auto_prune: false
```

### Module (`modules/desktop.yaml`)

```yaml
description: Desktop environment

packages:
  - i3
  - rofi
  - dunst
  - picom
  - feh
  - sxhkd
  - alacritty

post_install_hook: scripts/setup-desktop.sh
hook_behavior: once
```

---

## Commands

### Package Management
```sh
vcli install <pkg>           # install + track in config
vcli add <pkg1> <pkg2>       # install multiple
vcli remove <pkg>            # remove + untrack
vcli sync                    # apply config to system
vcli sync --dry-run          # preview changes
vcli sync --prune            # also remove undeclared packages
vcli update                  # xbps-install -Su
vcli merge                   # capture installed packages into config
vcli merge --services        # capture running runit services
vcli forget <pkg>            # stop tracking (keep installed)
vcli find <pkg>              # where is this package declared?
```

### Information
```sh
vcli status                  # config overview
vcli list                    # all declared packages + install status
vcli info <pkg>              # package details
vcli why <pkg>               # what depends on this package
vcli outdated                # packages with available updates
vcli orphans                 # unused auto-installed deps
vcli orphans --remove        # remove them
vcli doctor                  # full system health check
vcli log                     # operation history
vcli check                   # validate all package names before syncing
vcli validate                # check config integrity
```

### Modules
```sh
vcli module list             # show all modules
vcli module enable gaming    # enable a module
vcli module disable gaming   # disable a module
vcli module create mymodule  # create new module from template
```

### Desktop & Ricing
```sh
vcli desktop i3              # scaffold sxhkdrc, xprofile, helper scripts
vcli env init                # create ~/.config/environment.sh
vcli env set TERMINAL foot   # change default terminal everywhere
vcli env get                 # show all env vars
vcli theme ~/wall.jpg        # apply wallpaper + generate colors (pywal/wallust)
vcli theme --reload          # reload dunst, sxhkd without changing wallpaper
vcli rice create catppuccin  # create a new rice scaffold
vcli rice list               # show all rices
vcli dots catppuccin         # apply rice dotfiles (symlinks)
vcli dots catppuccin --dry-run
vcli rice current            # show active rice
```

### Git Sync (multi-machine)
```sh
vcli repo init               # init git for void-config
vcli repo push               # commit + push changes
vcli repo pull               # pull changes from remote
vcli repo clone              # clone existing config
vcli repo status             # git status
```

### Other
```sh
vcli save-config             # manual config backup
vcli restore-config          # interactive restore
vcli hooks list              # post-install hook status
vcli hooks reset <module>    # re-run hook on next sync
vcli edit                    # open config files in $EDITOR
vcli search                  # interactive package search (requires fzf)
vcli completions fish        # generate fish completions
vcli self-update             # rebuild vcli from source
```

---

## Shell Setup (fish)

Add to `~/.config/fish/functions/vcli.fish`:

```fish
function vcli
    set root_cmds sync install add remove update orphans doctor
    if contains -- $argv[1] $root_cmds
        sudo -E /usr/local/bin/vcli $argv
    else
        /usr/local/bin/vcli $argv
    end
end
```

Now `vcli sync` automatically uses sudo without you typing it. `HOME` is preserved so the config is always found.

For tab completion:
```sh
vcli completions fish > ~/.config/fish/completions/vcli.fish
```

---

## Multi-Machine Bootstrap

Store your config in a private git repo, then on any fresh Void install:

```sh
sudo xbps-install -Sy git
git clone git@github.com:yourusername/vcli.git
cd vcli
./bootstrap.sh
```

`bootstrap.sh` installs Rust, builds vcli, clones your config, and runs `vcli sync`.

Edit `bootstrap.sh` to point to your config repo:
```sh
VCLI_REPO="git@github.com:yourusername/vcli.git"
CONFIG_REPO="git@github.com:yourusername/void-config.git"
```

---

## Runit Services

Void uses runit instead of systemd. vcli handles services by:
- **Enable**: `ln -sf /etc/sv/<name> /var/service/`
- **Disable**: `rm -f /var/service/<name>`

Service names must match directories in `/etc/sv/`:
```yaml
services:
  enabled:
    - dbus
    - NetworkManager
    - bluetoothd
    - docker
    - cronie
```

---

## Environment Variables

| Variable | Default | Effect |
|----------|---------|--------|
| `VOID_CONFIG_DIR` | `~/.config/void-config` | Config location |
| `EDITOR` | `vi` | Used by `vcli edit` |
| `VCLI_SRC_DIR` | `~/vcli` | Used by `vcli self-update` |

---

## Building from Source

```sh
git clone https://github.com/elshaddoll-v/vcli.git
cd vcli
cargo build --release
sudo cp target/release/vcli /usr/local/bin/vcli
```

Requires Rust 1.70+. All dependencies are fetched by cargo automatically.

---

## Differences from dcli (Arch → Void)

| | dcli (Arch) | vcli (Void) |
|-|-------------|-------------|
| Package manager | pacman + AUR | xbps-install |
| Service manager | systemd | runit / sv |
| Config dir | `~/.config/arch-config` | `~/.config/void-config` |
| AUR support | Yes | No (use void-packages) |
| Lua scripting | Yes | Planned |
| TUI (ratatui) | Yes | Planned |

---

## License

0BSD — do whatever you want with it.

## Credits

Inspired by [dcli](https://gitlab.com/theblackdon/dcli) by Black Don.
