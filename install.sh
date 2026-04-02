#!/bin/bash
# vcli install script for Void Linux
# Usage: ./install.sh
set -e

INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
BINARY_NAME="vcli"

echo "==> vcli — declarative package manager for Void Linux"
echo ""

# ── Check we're on Void Linux ─────────────────────────────────────────────────
if ! command -v xbps-install &>/dev/null; then
    echo "ERROR: xbps-install not found."
    echo "vcli is designed for Void Linux only."
    exit 1
fi

# ── Rust ─────────────────────────────────────────────────────────────────────
if ! command -v cargo &>/dev/null; then
    [ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"
fi

if ! command -v cargo &>/dev/null; then
    echo "==> Rust not found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    source "$HOME/.cargo/env"
fi

echo "==> Rust: $(rustc --version)"
echo ""

# ── C compiler (required by Rust linker) ─────────────────────────────────────
if ! command -v cc &>/dev/null; then
    echo "==> C compiler not found. Installing gcc..."
    sudo xbps-install -Sy gcc
fi

# ── Build ─────────────────────────────────────────────────────────────────────
echo "==> Building vcli..."
cargo build --release

BINARY="target/release/${BINARY_NAME}"
if [ ! -f "$BINARY" ]; then
    echo "ERROR: Build failed."
    exit 1
fi

# ── Install ───────────────────────────────────────────────────────────────────
echo ""
echo "==> Installing to ${INSTALL_DIR}/${BINARY_NAME}..."
if [ -w "$INSTALL_DIR" ]; then
    cp "$BINARY" "${INSTALL_DIR}/${BINARY_NAME}"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
else
    sudo cp "$BINARY" "${INSTALL_DIR}/${BINARY_NAME}"
    sudo chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
fi

echo ""
echo "✓ vcli installed!"

# ── Install bundled modules ────────────────────────────────────────────────────
if [ -d "modules" ]; then
    echo "==> Installing bundled modules to /usr/share/vcli/modules..."
    sudo mkdir -p /usr/share/vcli/modules
    sudo cp -r modules/* /usr/share/vcli/modules/
    sudo mkdir -p /usr/share/vcli/scripts
    [ -d "scripts" ] && sudo cp -r scripts/* /usr/share/vcli/scripts/
    echo "✓ Modules installed"
fi
echo ""

# ── Install built-in modules ──────────────────────────────────────────────────
if [ -d "modules" ]; then
    echo "==> Installing built-in module library..."
    if [ -w "/usr/local/share" ]; then
        mkdir -p /usr/local/share/vcli/modules
        cp modules/*.yaml /usr/local/share/vcli/modules/
    else
        sudo mkdir -p /usr/local/share/vcli/modules
        sudo cp modules/*.yaml /usr/local/share/vcli/modules/
    fi
    echo "✓ $(ls modules/*.yaml | wc -l) built-in modules installed"
    echo ""
fi

# ── Optional deps ─────────────────────────────────────────────────────────────
echo "==> Optional dependencies:"
check() {
    if command -v "$1" &>/dev/null; then
        echo "  ✓ $1"
    else
        echo "  ✗ $1  →  xbps-install -S $2"
    fi
}
check "fzf"     "fzf"
check "wal"     "python3-pywal"
check "git"     "git"
check "flatpak" "flatpak"

echo ""
echo "==> Shell setup (fish):"
echo "    Create ~/.config/fish/functions/vcli.fish with:"
echo ""
echo "    function vcli"
echo "        set root_cmds sync install add remove update orphans doctor"
echo "        if contains -- \$argv[1] \$root_cmds"
echo "            sudo -E /usr/local/bin/vcli \$argv"
echo "        else"
echo "            /usr/local/bin/vcli \$argv"
echo "        end"
echo "    end"
echo ""
echo "==> Get started:"
echo "    vcli init          # create ~/.config/void-config/"
echo "    vcli merge         # capture installed packages"
echo "    vcli sync          # apply config to system"
echo "    vcli --help        # all commands"
echo ""
