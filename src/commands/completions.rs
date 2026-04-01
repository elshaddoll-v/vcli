//! Generate shell completions for vcli.
use anyhow::Result;
use colored::*;

pub fn run(shell: &str) -> Result<()> {
    match shell {
        "fish" => print!("{}", FISH_COMPLETIONS),
        "bash" => print!("{}", BASH_COMPLETIONS),
        "zsh"  => print!("{}", ZSH_COMPLETIONS),
        _ => anyhow::bail!("Unknown shell '{}'. Supported: fish, bash, zsh", shell),
    }
    Ok(())
}

/// Print install instructions for the shell
pub fn install_instructions(shell: &str) {
    match shell {
        "fish" => {
            println!("{}", "To install fish completions:".blue());
            println!("  vcli completions fish > ~/.config/fish/completions/vcli.fish");
        }
        "bash" => {
            println!("{}", "To install bash completions:".blue());
            println!("  vcli completions bash > ~/.bash_completion.d/vcli");
            println!("  source ~/.bash_completion.d/vcli");
        }
        "zsh" => {
            println!("{}", "To install zsh completions:".blue());
            println!("  vcli completions zsh > ~/.zfunc/_vcli");
            println!("  fpath=(~/.zfunc $fpath)");
        }
        _ => {}
    }
}

const FISH_COMPLETIONS: &str = r#"# vcli fish completions
# Install: vcli completions fish > ~/.config/fish/completions/vcli.fish

set -l commands sync install add remove status update merge find forget validate check list \
    info why outdated orphans doctor log \
    desktop env theme rice dots \
    module hooks repo edit search save-config restore-config self-update completions init fish-setup

complete -c vcli -f

# Top-level commands
complete -c vcli -n "not __fish_seen_subcommand_from $commands" -a "sync"         -d "Sync system to config"
complete -c vcli -n "not __fish_seen_subcommand_from $commands" -a "install add"  -d "Install package(s)"
complete -c vcli -n "not __fish_seen_subcommand_from $commands" -a "remove"       -d "Remove package(s)"
complete -c vcli -n "not __fish_seen_subcommand_from $commands" -a "status"       -d "Show config status"
complete -c vcli -n "not __fish_seen_subcommand_from $commands" -a "update"       -d "System update"
complete -c vcli -n "not __fish_seen_subcommand_from $commands" -a "merge"        -d "Capture installed packages"
complete -c vcli -n "not __fish_seen_subcommand_from $commands" -a "find"         -d "Find where package is declared"
complete -c vcli -n "not __fish_seen_subcommand_from $commands" -a "forget"       -d "Stop tracking a package"
complete -c vcli -n "not __fish_seen_subcommand_from $commands" -a "validate"     -d "Validate config"
complete -c vcli -n "not __fish_seen_subcommand_from $commands" -a "check"        -d "Check all package names exist"
complete -c vcli -n "not __fish_seen_subcommand_from $commands" -a "list"         -d "List declared packages"
complete -c vcli -n "not __fish_seen_subcommand_from $commands" -a "module"       -d "Module management"
complete -c vcli -n "not __fish_seen_subcommand_from $commands" -a "hooks"        -d "Hook management"
complete -c vcli -n "not __fish_seen_subcommand_from $commands" -a "repo"         -d "Git repo management"
complete -c vcli -n "not __fish_seen_subcommand_from $commands" -a "edit"         -d "Edit config files"
complete -c vcli -n "not __fish_seen_subcommand_from $commands" -a "search"       -d "Interactive package search"
complete -c vcli -n "not __fish_seen_subcommand_from $commands" -a "save-config"  -d "Backup config"
complete -c vcli -n "not __fish_seen_subcommand_from $commands" -a "self-update"  -d "Update vcli"
complete -c vcli -n "not __fish_seen_subcommand_from $commands" -a "init"         -d "Initialize void-config"

# sync flags
complete -c vcli -n "__fish_seen_subcommand_from sync" -l dry-run      -d "Preview only"
complete -c vcli -n "__fish_seen_subcommand_from sync" -l prune        -d "Remove undeclared packages"
complete -c vcli -n "__fish_seen_subcommand_from sync" -l force        -d "Skip confirmation"
complete -c vcli -n "__fish_seen_subcommand_from sync" -l no-backup    -d "Skip backup"
complete -c vcli -n "__fish_seen_subcommand_from sync" -l no-hooks     -d "Skip hooks"
complete -c vcli -n "__fish_seen_subcommand_from sync" -l auto-commit  -d "Auto git commit"

# install/add — complete with xbps package names
complete -c vcli -n "__fish_seen_subcommand_from install add" -a "(xbps-query -Rs '' 2>/dev/null | awk '{print \$2}' | sed 's/-[0-9].*//')" -d "package"

# remove — complete with declared packages
complete -c vcli -n "__fish_seen_subcommand_from remove" -a "(vcli list 2>/dev/null | awk '/✓|✗/{print \$2}')"

# module subcommands
complete -c vcli -n "__fish_seen_subcommand_from module; and not __fish_seen_subcommand_from list enable disable create" -a "list"    -d "List modules"
complete -c vcli -n "__fish_seen_subcommand_from module; and not __fish_seen_subcommand_from list enable disable create" -a "enable"  -d "Enable module"
complete -c vcli -n "__fish_seen_subcommand_from module; and not __fish_seen_subcommand_from list enable disable create" -a "disable" -d "Disable module"
complete -c vcli -n "__fish_seen_subcommand_from module; and not __fish_seen_subcommand_from list enable disable create" -a "create"  -d "Create module"

# hooks subcommands
complete -c vcli -n "__fish_seen_subcommand_from hooks; and not __fish_seen_subcommand_from list reset skip run" -a "list reset skip run"

# repo subcommands
complete -c vcli -n "__fish_seen_subcommand_from repo; and not __fish_seen_subcommand_from init clone push pull status" -a "init clone push pull status"

# completions shells
complete -c vcli -n "__fish_seen_subcommand_from completions" -a "fish bash zsh" -d "shell"

# desktop / env / theme / rice / dots
complete -c vcli -n "__fish_seen_subcommand_from desktop" -a "i3 dwm bspwm awesome openbox" -d "window manager"
complete -c vcli -n "__fish_seen_subcommand_from env" -a "get set unset init edit" -d "action"
complete -c vcli -n "__fish_seen_subcommand_from rice" -a "list apply create current remove" -d "action"
complete -c vcli -n "__fish_seen_subcommand_from dots" -l dry-run -d "Preview only"
complete -c vcli -n "__fish_seen_subcommand_from dots" -l force -d "Overwrite with backup"
complete -c vcli -n "__fish_seen_subcommand_from theme" -l reload -d "Reload apps only"

# info / why / outdated / orphans / doctor / log
complete -c vcli -n "__fish_seen_subcommand_from info why" -a "(xbps-query -l 2>/dev/null | awk '{print \$2}' | sed 's/-[0-9].*//')" -d "installed package"
complete -c vcli -n "__fish_seen_subcommand_from orphans" -l remove -d "Remove all orphans"
complete -c vcli -n "__fish_seen_subcommand_from log" -s n -l lines -d "Number of lines"

# global flags
complete -c vcli -s j -l json -d "JSON output"
"#;

const BASH_COMPLETIONS: &str = r#"# vcli bash completions
# Install: vcli completions bash >> ~/.bashrc  OR  source <(vcli completions bash)

_vcli() {
    local cur prev words cword
    _init_completion || return

    local commands="sync install add remove status update merge find forget validate check list module hooks repo edit search save-config restore-config self-update completions init"

    case $prev in
        vcli)
            COMPREPLY=($(compgen -W "$commands" -- "$cur"))
            return ;;
        install|add)
            local pkgs=$(xbps-query -Rs "$cur" 2>/dev/null | awk '{print $2}' | sed 's/-[0-9].*//')
            COMPREPLY=($(compgen -W "$pkgs" -- "$cur"))
            return ;;
        module)
            COMPREPLY=($(compgen -W "list enable disable create" -- "$cur"))
            return ;;
        hooks)
            COMPREPLY=($(compgen -W "list reset skip run" -- "$cur"))
            return ;;
        repo)
            COMPREPLY=($(compgen -W "init clone push pull status" -- "$cur"))
            return ;;
        completions)
            COMPREPLY=($(compgen -W "fish bash zsh" -- "$cur"))
            return ;;
        sync)
            COMPREPLY=($(compgen -W "--dry-run --prune --force --no-backup --no-hooks --auto-commit" -- "$cur"))
            return ;;
    esac

    COMPREPLY=($(compgen -W "$commands" -- "$cur"))
}

complete -F _vcli vcli
"#;

const ZSH_COMPLETIONS: &str = r#"#compdef vcli
# vcli zsh completions
# Install: vcli completions zsh > ~/.zfunc/_vcli  (add ~/.zfunc to fpath)

_vcli() {
    local state

    _arguments \
        '(-j --json)'{-j,--json}'[JSON output]' \
        '1: :->command' \
        '*: :->args'

    case $state in
        command)
            local commands=(
                'sync:Sync system to config'
                'install:Install package(s)'
                'add:Install package(s) (shorthand)'
                'remove:Remove package(s)'
                'status:Show config status'
                'update:System update'
                'merge:Capture installed packages'
                'find:Find where package is declared'
                'forget:Stop tracking a package'
                'validate:Validate config'
                'check:Check all package names'
                'list:List declared packages'
                'module:Module management'
                'hooks:Hook management'
                'repo:Git repo management'
                'edit:Edit config files'
                'search:Interactive package search'
                'save-config:Backup config'
                'self-update:Update vcli'
                'init:Initialize void-config'
                'completions:Generate shell completions'
            )
            _describe 'command' commands ;;
        args)
            case $words[2] in
                module) _values 'action' list enable disable create ;;
                hooks)  _values 'action' list reset skip run ;;
                repo)   _values 'action' init clone push pull status ;;
                completions) _values 'shell' fish bash zsh ;;
                sync)
                    _arguments \
                        '--dry-run[Preview only]' \
                        '--prune[Remove undeclared]' \
                        '--force[Skip confirmation]' \
                        '--no-backup[Skip backup]' \
                        '--no-hooks[Skip hooks]' ;;
            esac ;;
    esac
}

_vcli
"#;
