#!/bin/bash
# vcli install script for Void Linux
set -e

INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
BINARY_NAME="vcli"

echo "==> vcli installer for Void Linux"
echo ""

# Check for Rust
if ! command -v cargo &>/dev/null; then
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    fi
fi

if ! command -v cargo &>/dev/null; then
    echo "==> Rust not found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    source "$HOME/.cargo/env"
fi

echo "==> Rust found: $(rustc --version)"
echo ""

# Build
echo "==> Building vcli (release mode)..."
cargo build --release

BINARY="target/release/${BINARY_NAME}"

if [ ! -f "$BINARY" ]; then
    echo "ERROR: Build failed — binary not found at $BINARY"
    exit 1
fi

# Install
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
echo "✓ vcli installed to ${INSTALL_DIR}/${BINARY_NAME}"
echo ""

# Check optional dependencies
echo "==> Checking optional dependencies..."
check_opt() {
    if command -v "$1" &>/dev/null; then
        echo "  ✓ $1"
    else
        echo "  ✗ $1 (optional — install with: xbps-install -S $2)"
    fi
}

check_opt "fzf" "fzf"
check_opt "flatpak" "flatpak"
check_opt "git" "git"

echo ""
echo "==> Get started:"
echo "    vcli init          # Initialize void-config"
echo "    vcli sync          # Sync packages to match config"
echo "    vcli install vim   # Install a package and track it"
echo "    vcli help          # Show all commands"
echo ""
