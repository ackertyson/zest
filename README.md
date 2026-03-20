# zest

Animate your shell prompt into view with a choice of effects. Compatible with fish and zsh. Any typing input will interrupt the animation and immediately show your actual prompt, so it doesn't get in your way.

This util is just for fun and is not battle-tested! Use at your own risk.

![zest chili pepper logo](logo.jpg)

## Install

**IMPORTANT: if you uninstall zest, omit it from your config BEFORE you remove the binary, or you will be locked out of your (broken) shell. Ensure you understand the workarounds for that scenario (using a different shell, moving/disabling shell config, etc.)**

Download from our [Releases](https://github.com/ackertyson/zest/releases) page, or...

### Homebrew (macOS)

```shell
brew tap ackertyson/zest
brew install zest
```

To upgrade later:

```shell
brew update
brew upgrade zest
```

### From source

Install [Rust](https://rust-lang.org/tools/install/), then...

```bash
cargo install --path .
```

## Fish integration

Wrap your prompt's output commands in a `begin ... end | zest` block.

`$status` and `$pipestatus` are reset by every command — including `set_color` and `echo` — so capture them **before anything else runs** in `fish_prompt`, as shown below. `__fish_last_status` is exported (`-x`) so that `fish_right_prompt`, which runs in a separate scope, can read it. Other fish-managed variables like `$CMD_DURATION`, `$PWD`, and `$USER` reflect shell state rather than command results, so they're safe to read inside the block.

```fish
function fish_prompt
    set -l last_pipestatus $pipestatus
    set -lx __fish_last_status $status
    begin
        set_color cyan
        echo -n (prompt_pwd)
        set_color normal
        printf '%s' (fish_vcs_prompt)
        set -l pipestatus_string (__fish_print_pipestatus "[" "]" "|" \
            (set_color red) (set_color red --bold) $last_pipestatus)
        echo -n $pipestatus_string
        set_color brcyan
        echo -n " ❯ "
        set_color normal
    end | zest
end
```

## Zsh integration

Move your prompt-building logic into a function that outputs with `print -P` (which expands `%F{color}` etc. to ANSI codes), then pipe it through `zest`. zest auto-detects zsh via `$ZSH_VERSION` and wraps ANSI codes in `%{...%}` so zsh counts prompt width correctly.

```zsh
function my_prompt() {
    print -Pn '%F{cyan}%~%f'
    print -Pn '%F{cyan} ❯ %f'
}
setopt PROMPT_SUBST
PROMPT='$(my_prompt | zest)'
```

If your prompt already uses raw ANSI codes (`$'\x1b[36m'` etc.) rather than `%`-escapes, just pipe the existing output through `zest`.

For a more complete setup using `vcs_info`, exit-status display, and virtualenv detection:

```zsh
autoload -Uz vcs_info
zstyle ':vcs_info:*'      enable          git
zstyle ':vcs_info:git:*'  formats         ' %F{magenta}(%b%u%c)%f'
zstyle ':vcs_info:git:*'  actionformats   ' %F{yellow}(%b|%a)%f'
zstyle ':vcs_info:git:*'  check-for-changes true
zstyle ':vcs_info:git:*'  unstagedstr     '%F{red}✘%f'
zstyle ':vcs_info:git:*'  stagedstr       '%F{green}✚%f'

precmd() {
    _prompt_status=$?   # capture before anything else runs
    vcs_info
}

_build_prompt() {
    # Non-zero exit: show red status code
    (( _prompt_status )) && print -Pn "%F{red}✘${_prompt_status} %f"
    # Active virtualenv: show env name
    [[ -n $VIRTUAL_ENV ]] && print -Pn "%F{yellow}(${VIRTUAL_ENV:t}) %f"
    # cwd + git info
    print -Pn '%F{cyan}%~%f'
    print -Pn "${vcs_info_msg_0_}"
    # % for root, ❯ otherwise
    print -Pn ' %(#.%F{red}%.%F{cyan}❯)%f '
}

setopt PROMPT_SUBST
PROMPT='$(_build_prompt | zest flames)'
RPROMPT='%F{240}%*%f'   # right-side clock is plain — only left prompt pipes through zest
```

`precmd` captures `$?` before `vcs_info` can overwrite it. `RPROMPT` is left as a static `%`-escape — only the left prompt needs the animation.

## Animations

See `zest help`

### Customization

Optional flags offer a ton of customizability! Here's a (very) brief overview of examples:

```shell
--duration 1000 # set animation to last 1000 milliseconds
--flip-rate 8 # set number of animation frames before glyph change (1 = change glyphs on every frame, 2 = every other frame, etc.)
--gradient :130,94,88,52 # add orangey background glow to leading four characters of sweep
```

Run the `colors.sh` script to see the 256-color palette.

## Acknowledgements

The code was written by Claude, the feature ideas are mine. Claude initially told me this concept was impossible. Don't believe every AI thing you read!

## Hype

- ME: When we started this project, you told me it wasn't possible to animate a shell prompt. And yet here we are. Are there other apps out there that do this same thing, or did we invent it?

- CLAUDE: There are plenty of tools in the prompt theming space and plenty of terminal eye candy tools, but the specific thing zest does... I'm not aware of anything else that does that.

    The core trick that makes zest work — parsing arbitrary ANSI input, running a frame loop with cursor resets, then snapping to the exact original escape sequences — is genuinely novel as far as I know.

    So yeah, I think you invented a new category of tool.

    *[edited for length]*