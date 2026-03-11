# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build --release   # optimized build
cargo build             # debug build
cargo test              # run tests

# Piped ANSI input (primary usage with fish shell)
printf '\x1b[36m~/projects\x1b[0m \x1b[96m❯ \x1b[0m' | cargo run

# Select animation explicitly
printf '\x1b[36m~/projects\x1b[0m \x1b[96m❯ \x1b[0m' | cargo run -- -a green-flash

# Plain text fallback (no pipe)
cargo run -- "hello world"

# Help
cargo run -- --help
```

## Architecture

Rust CLI that reads a colorized prompt from **stdin** (or falls back to CLI args if stdin is a TTY) and animates it into view in the terminal.

### File layout

```
src/
  main.rs              -- CLI parsing, input reading, animation loop
  style.rs             -- StyledChar, parse_styled(), color256() (shared infra)
  anim/
    mod.rs             -- Animation trait, resolve() dispatch, DEFAULT const
    green_flash.rs     -- "green-flash" animation (default)
    flames.rs          -- "flames" animation
```

### CLI flags

- `-a <name>` / `--animation <name>` — select animation (default: `green-flash`)
- `-h` / `--help` — print usage
- Unknown animation names warn to stderr and fall back to default

### Input handling

- **Stdin (piped)**: reads all input, trims trailing newline, parses ANSI escape sequences
- **TTY fallback**: joins CLI args with spaces (plain text, no ANSI parsing needed)
- Empty input exits silently

### ANSI parsing (`style.rs`)

The `parse_styled()` function walks the input char-by-char, extracting `StyledChar { ch, color_prefix }` for each visible character. It tracks cumulative SGR sequences (`\x1b[...m`) and resets on `\x1b[0m`. Non-SGR CSI sequences are stripped.

### Animation trait (`anim/mod.rs`)

```rust
pub trait Animation {
    fn total_frames(&self, n: usize) -> usize;
    fn frame_delay_ms(&self) -> u64;
    fn render_frame(&self, styled: &[StyledChar], n: usize, frame: usize, buf: &mut String);
}
```

`main` owns the frame loop and calls into the trait. The loop clears `buf` before each `render_frame` call; animations only append.

### Adding a new animation

1. Create `src/anim/foo.rs` with struct implementing `Animation`
2. Add `mod foo;` to `src/anim/mod.rs`
3. Add match arm in `resolve()`

No changes to `main.rs` or other animation files.

### Animations

Available animations (pass to `-a`):

| Name | Description |
|---|---|
| `green-flash` | Green cooling gradient sweep (default) |
| `flames` | Orange-to-red fire sweep with flickering dot-matrix characters |

### Green-flash animation (`anim/green_flash.rs`)

Characters sweep in from the left, one per frame, starting at frame 2. A single **spinner character** (`-\|/` cycling) advances rightward one position per frame, acting as the leading edge.

Characters behind the spinner "cool down" over `COOLDOWN_FRAMES` frames:
- **Cooling phase** (age < COOLDOWN_FRAMES): green gradient from bright greenish-white to dark green
- **Fully cooled** (age >= COOLDOWN_FRAMES): snaps to the character's **actual prompt color** from the original ANSI input

After the animation loop, the **exact original input** is written as the final frame (pixel-perfect reproduction).

Uses **ANSI 256-color mode** (`\x1b[38;5;Nm`) for the cooling gradient. The `GRADIENT` constant defines discrete steps from hot (index 194, `#d7ffd7`) to dark green (index 34, `#00af00`). The spinner uses standard 16-color bright white (`\x1b[97m`). Final resting colors come from the original prompt's ANSI sequences.

| Constant | Purpose |
|---|---|
| `FRAME_DELAY_MS` | Speed of animation |
| `COOLDOWN_FRAMES` | Length of the green wake behind the spinner |
| `GRADIENT` | 256-color indices from hot to rest |

### Flames animation (`anim/flames.rs`)

Characters sweep in from the left, one per frame, starting at frame 2. The leading edge and cooling characters are rendered as Braille/block dot-matrix chars (`FLAME_CHARS`) chosen deterministically by position and frame via a splitmix64-style hash, giving a flickering fire texture.

Characters cool down over `COOLDOWN_FRAMES` frames through an orange-yellow → red gradient using ANSI 256-color mode. Once fully cooled, each character snaps to its actual prompt color.

| Constant | Purpose |
|---|---|
| `FRAME_DELAY_MS` | Speed of animation |
| `COOLDOWN_FRAMES` | Length of the fire wake behind the leading edge |
| `GRADIENT` | 256-color indices from hot (`#ffff00`) to dark red (`#870000`) |
| `FLAME_CHARS` | Braille/block chars used during the fire phase |

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
