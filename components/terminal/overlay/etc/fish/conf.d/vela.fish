# Vela: defaults for fish (system-wide). Your own settings go in ~/.config/fish/config.fish
# and always win over these. Autosuggestions and syntax highlighting are built into fish.
status is-interactive; or exit

# No banner: the prompt alone is enough.
set -g fish_greeting

# Prompt: starship with the Vela config, unless you have your own ~/.config/starship.toml.
if command -q starship; and test "$TERM" != dumb
    if not set -q STARSHIP_CONFIG; and not test -e ~/.config/starship.toml
        set -gx STARSHIP_CONFIG /usr/share/vela/terminal/starship.toml
    end
    starship init fish | source
end

# Colours that read well on both Vela light and dark terminal schemes (ANSI names, so they
# follow the colour scheme). Set only if the user never chose a fish theme.
if not set -q __vela_fish_colors_v1
    set -U __vela_fish_colors_v1 1
    set -U fish_color_command blue
    set -U fish_color_param normal
    set -U fish_color_quote green
    set -U fish_color_redirection cyan
    set -U fish_color_end magenta
    set -U fish_color_error red
    set -U fish_color_comment brblack
    set -U fish_color_operator cyan
    set -U fish_color_escape magenta
    set -U fish_color_autosuggestion brblack
    set -U fish_color_valid_path --underline
    set -U fish_pager_color_prefix blue --bold
    set -U fish_pager_color_progress brblack
    set -U fish_pager_color_description brblack
end

# Short commands (fish "abbreviations": they expand as you type, so you see the real command).
abbr -a ll 'ls -lh'
abbr -a la 'ls -lAh'
abbr -a apri xdg-open
abbr -a aggiorna 'sudo apt update && sudo apt full-upgrade'
abbr -a installa 'sudo apt install'
abbr -a cerca 'apt search'
abbr -a spazio 'df -h -x tmpfs -x devtmpfs -x squashfs'
abbr -a ipc 'ip -c -br address'

# "ia <richiesta>": opens the Terminale IA with your request already filled in.
function ia --description 'Chiedi al Terminale IA (es. ia libera spazio sul disco)'
    if not command -q vela-ai-terminal
        echo "Il Terminale IA non è installato." >&2
        return 1
    end
    if test (count $argv) -eq 0
        vela-ai-terminal --workdir $PWD >/dev/null 2>&1 &
    else
        vela-ai-terminal --workdir $PWD --intent (string join ' ' -- $argv) >/dev/null 2>&1 &
    end
    disown 2>/dev/null
end

# Unknown command: plain Italian message, plus the package that provides it (if known).
function fish_command_not_found
    set -l cmd $argv[1]
    if string match -q 'en*' -- "$LANG"
        printf "%s: command not found\n" $cmd >&2
    else
        printf "%s: comando non trovato\n" $cmd >&2
    end
    if test -x /usr/lib/command-not-found
        /usr/lib/command-not-found -- $cmd 2>&1 | string match -rv '^\s*$' >&2
    end
end
