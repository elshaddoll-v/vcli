#!/bin/bash
# Post-install hook for zsh shell module

ACTUAL_USER="${SUDO_USER:-$USER}"
USER_HOME=$(getent passwd "$ACTUAL_USER" | cut -d: -f6)

echo "Setting up zsh for $ACTUAL_USER..."

# Set zsh as default shell
ZSH_PATH=$(which zsh)
if ! grep -q "$ZSH_PATH" /etc/shells; then
    echo "$ZSH_PATH" >> /etc/shells
fi
chsh -s "$ZSH_PATH" "$ACTUAL_USER"

# Create .zshrc
ZSHRC="$USER_HOME/.zshrc"
if [ ! -f "$ZSHRC" ]; then
    cat > "$ZSHRC" << 'ZSHEOF'
# Load plugins
source /usr/share/zsh/plugins/zsh-autosuggestions/zsh-autosuggestions.zsh 2>/dev/null || true
source /usr/share/zsh/plugins/zsh-syntax-highlighting/zsh-syntax-highlighting.zsh 2>/dev/null || true
source /usr/share/zsh/plugins/zsh-history-substring-search/zsh-history-substring-search.zsh 2>/dev/null || true

# Load environment
[ -f ~/.config/environment.sh ] && source ~/.config/environment.sh

# Starship prompt
eval "$(starship init zsh)"

# Zoxide
eval "$(zoxide init zsh)"

# History
HISTSIZE=10000
SAVEHIST=10000
HISTFILE=~/.zsh_history
setopt HIST_IGNORE_DUPS
setopt SHARE_HISTORY
ZSHEOF
    chown "$ACTUAL_USER:$ACTUAL_USER" "$ZSHRC"
fi

echo "✓ Zsh configured for $ACTUAL_USER"
