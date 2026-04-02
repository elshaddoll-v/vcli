#!/bin/bash
# Post-install hook for fish shell module
# Sets fish as default shell and configures starship + zoxide

USER_HOME=$(getent passwd "${SUDO_USER:-$USER}" | cut -d: -f6)
ACTUAL_USER="${SUDO_USER:-$USER}"

echo "Setting up fish shell for $ACTUAL_USER..."

# Set fish as default shell
FISH_PATH=$(which fish)
if ! grep -q "$FISH_PATH" /etc/shells; then
    echo "$FISH_PATH" >> /etc/shells
fi
chsh -s "$FISH_PATH" "$ACTUAL_USER"

# Create fish config directory
mkdir -p "$USER_HOME/.config/fish/functions"
mkdir -p "$USER_HOME/.config/fish/completions"

# Add starship to fish config
FISH_CONFIG="$USER_HOME/.config/fish/config.fish"
if ! grep -q "starship" "$FISH_CONFIG" 2>/dev/null; then
    echo 'starship init fish | source' >> "$FISH_CONFIG"
fi

# Add zoxide to fish config
if ! grep -q "zoxide" "$FISH_CONFIG" 2>/dev/null; then
    echo 'zoxide init fish | source' >> "$FISH_CONFIG"
fi

# Add vcli fish function
cat > "$USER_HOME/.config/fish/functions/vcli.fish" << 'FISHEOF'
function vcli
    set root_cmds sync install add remove update orphans doctor
    if contains -- $argv[1] $root_cmds
        sudo -E /usr/local/bin/vcli $argv
    else
        /usr/local/bin/vcli $argv
    end
end
FISHEOF

# Generate vcli completions
/usr/local/bin/vcli completions fish > "$USER_HOME/.config/fish/completions/vcli.fish" 2>/dev/null || true

chown -R "$ACTUAL_USER:$ACTUAL_USER" "$USER_HOME/.config/fish"

echo "✓ Fish shell configured for $ACTUAL_USER"
echo "  Log out and back in to use fish as default shell"
