# zest

Animate your terminal prompt into view with a choice of effects. The animation is written directly to `/dev/tty`, then the final prompt is emitted on `stdout`, compatible with fish and zsh prompt mechanics.

This util is just for fun and is not battle-tested! Use at your own risk.

![zest chili pepper logo](logo.jpg)

## Install

Install [Rust](https://rust-lang.org/tools/install/), then...

```bash
cargo install --path .
```

## Fish integration

Wrap your prompt's output commands in a `begin ... end | zest` block. Any variables you need to capture before output (e.g. `$pipestatus`, `$status`) should be set before the block.

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

Each time a new prompt renders, the selected animation fires and then settles into the configured prompt.

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

## Animations

See `zest help`