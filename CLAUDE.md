# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build --release   # optimized build
cargo build             # debug build
cargo test              # run tests

# Piped ANSI input (primary usage with fish shell)
printf '\x1b[36m~/projects\x1b[0m \x1b[96m❯ \x1b[0m' | cargo run

# Plain text fallback (no pipe)
cargo run -- "hello world"
```

## Architecture

Single-file Rust CLI (`src/main.rs`). Reads a colorized prompt from **stdin** (or falls back to CLI args if stdin is a TTY) and animates it into view in the terminal.

### Input handling

- **Stdin (piped)**: reads all input, trims trailing newline, parses ANSI escape sequences
- **TTY fallback**: joins CLI args with spaces (plain text, no ANSI parsing needed)
- Empty input exits silently

### ANSI parsing

The `parse_styled()` function walks the input char-by-char, extracting `StyledChar { ch, color_prefix }` for each visible character. It tracks cumulative SGR sequences (`\x1b[...m`) and resets on `\x1b[0m`. Non-SGR CSI sequences are stripped.

### Animation model

Characters sweep in from the left, one per frame, starting at frame 2. A single **spinner character** (`-\|/` cycling) advances rightward one position per frame, acting as the leading edge.

Characters behind the spinner "cool down" over `COOLDOWN_FRAMES` frames:
- **Cooling phase** (age < COOLDOWN_FRAMES): green gradient from bright greenish-white to dark green
- **Fully cooled** (age >= COOLDOWN_FRAMES): snaps to the character's **actual prompt color** from the original ANSI input

After the animation loop, the **exact original input** is written as the final frame (pixel-perfect reproduction).

### Color scheme

Uses **ANSI 256-color mode** (`\x1b[38;5;Nm`) for the cooling gradient. The `GRADIENT` constant defines discrete steps from hot (index 194, `#d7ffd7`) to dark green (index 34, `#00af00`). The spinner uses standard 16-color bright white (`\x1b[97m`). Final resting colors come from the original prompt's ANSI sequences.

### Key constants to tune

| Constant | Purpose |
|---|---|
| `FRAME_DELAY_MS` | Speed of animation |
| `COOLDOWN_FRAMES` | Length of the green wake behind the spinner |
| `GRADIENT` | 256-color indices from hot to rest |

### Fish shell integration

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
