# zest

Animates your terminal prompt into view with a choice of effects. The animation is written directly to `/dev/tty`, then the final prompt is emitted on stdout — compatible with fish and zsh prompt mechanics.

## Install

```bash
cargo install --path .
```

## Usage

Pipe any ANSI-colored text to `zest` to animates it:

```bash
printf '\x1b[36m~/projects\x1b[0m \x1b[96m❯ \x1b[0m' | zest
```

Select an animation explicitly:

```bash
printf '\x1b[36m~/projects\x1b[0m \x1b[96m❯ \x1b[0m' | zest green-flash
```

Plain text fallback (no pipe):

```bash
zest "hello world"
```

## Fish shell integration

Wrap your prompt's output commands in a `begin ... end | zest` block. Any variables you need to capture before output (e.g. `$pipestatus`, `$status`) should be set before the block as usual — fish scoping means they'll still be readable inside it.

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

Each time a new prompt renders, the text sweeps in with the selected animation and settles into its original colors.

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

If your prompt already uses raw ANSI codes (`$'\x1b[36m'` etc.) rather than `%`-escapes, it works the same way — just pipe the output through `zest`.

## Animations

| Name          | Description                                                                          |
|---------------|--------------------------------------------------------------------------------------|
| `green-flash` | Characters sweep in left-to-right with a green cooling gradient (default)            |
| `flames`      | Characters sweep in as flickering dot-matrix fire, cooling from orange-yellow to red |
