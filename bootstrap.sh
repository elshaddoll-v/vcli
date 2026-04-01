#!/bin/bash
# vcli bootstrap — restore your Void Linux system on a fresh install
# Edit the two variables below, then run this script once.
set -e

# ── Edit these ────────────────────────────────────────────────────────────────
VCLI_REPO="git@github.com:elshaddoll-v/vcli.git"
CONFIG_REPO="git@github.com:elshaddoll-v/void-config.git"
# ─────────────────────────────────────────────────────────────────────────────

echo "==> vcli bootstrap"
echo ""

# Check we're on Void
if ! command -v xbps-install &>/dev/null; then
    echo "ERROR: This script is for Void Linux only."
    exit 1
fi

# Install git and curl if missing
echo "==> Installing dependencies..."
sudo xbps-install -Sy git curl

# Set up SSH key if not present
if [ ! -f "$HOME/.ssh/id_ed25519" ]; then
    echo ""
    echo "==> No SSH key found. Generating one..."
    ssh-keygen -t ed25519 -C "$(whoami)@$(hostname)" -N "" -f "$HOME/.ssh/id_ed25519"
    echo ""
    echo "Your public key:"
    echo ""
    cat "$HOME/.ssh/id_ed25519.pub"
    echo ""
    echo "Add this key to GitHub: https://github.com/settings/keys"
    echo "Then press ENTER to continue..."
    read -r
fi

# Clone vcli
if [ ! -d "$HOME/vcli" ]; then
    echo "==> Cloning vcli..."
    git clone "$VCLI_REPO" "$HOME/vcli"
fi

# Build and install vcli
echo "==> Installing vcli..."
cd "$HOME/vcli"
chmod +x install.sh
./install.sh

# Clone void-config
CONFIG_DIR="${VOID_CONFIG_DIR:-$HOME/.config/void-config}"
if [ ! -d "$CONFIG_DIR" ]; then
    echo "==> Cloning void-config..."
    git clone "$CONFIG_REPO" "$CONFIG_DIR"
else
    echo "==> void-config already exists at $CONFIG_DIR"
fi

# Set up fish function
if command -v fish &>/dev/null; then
    mkdir -p "$HOME/.config/fish/functions"
    cat > "$HOME/.config/fish/functions/vcli.fish" << 'FISHEOF'
function vcli
    set root_cmds sync install add remove update orphans doctor
    if contains -- $argv[1] $root_cmds
        sudo -E /usr/local/bin/vcli $argv
    else
        /usr/local/bin/vcli $argv
    end
end
FISHEOF
    echo "==> Fish function installed"
fi

# Install completions
if command -v fish &>/dev/null; then
    mkdir -p "$HOME/.config/fish/completions"
    /usr/local/bin/vcli completions fish > "$HOME/.config/fish/completions/vcli.fish" 2>/dev/null || true
fi

echo ""
echo "==> Syncing system..."
sudo -E /usr/local/bin/vcli sync

echo ""
echo "✓ Bootstrap complete!"
echo ""
echo "Your system has been restored."
echo "Run 'vcli status' to see your config."
