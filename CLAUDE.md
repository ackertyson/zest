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
printf '\x1b[36m~/projects\x1b[0m \x1b[96m❯ \x1b[0m' | cargo run -- sprout

# Select animation with color
printf '\x1b[36m~/projects\x1b[0m \x1b[96m❯ \x1b[0m' | cargo run -- flames pink
printf '\x1b[36m~/projects\x1b[0m \x1b[96m❯ \x1b[0m' | cargo run -- matrix blue

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
  shell.rs             -- wrap_ansi_for_zsh(), is_zsh() (shell-specific logic)
  anim/
    mod.rs             -- Animation trait, resolve() dispatch, cooldown_color(), DEFAULT const
    sprout.rs          -- "sprout" animation (default)
    flames.rs          -- "flames" animation with color variants (orange/blue/green/purple/pink)
    matrix.rs          -- "matrix" animation
    scan.rs            -- "scan" animation
    lightning.rs       -- "lightning" animation
```

### CLI

- `zest [ANIMATION [COLOR]]` — optional positional args select animation and color variant (default: `sprout`)
- `--zsh` — wrap ANSI codes for zsh prompt width
- `-h` / `--help` — print usage
- Unknown animation names are treated as fallback text; unrecognized colors fall back to the animation's default color

### Input handling

- **Stdin (piped)**: reads all input, trims trailing newline, parses ANSI escape sequences
- **TTY fallback**: joins CLI args with spaces (plain text, no ANSI parsing needed)
- Empty input exits silently

### ANSI parsing (`style.rs`)

The `parse_styled()` function walks the input char-by-char, extracting `StyledChar { ch, color_prefix }` for each visible character. It tracks cumulative SGR sequences (`\x1b[...m`) and resets on `\x1b[0m`. Non-SGR CSI sequences are stripped.

### Animation trait (`anim/mod.rs`)

```rust
pub trait Animation {
    fn cooldown_frames(&self) -> usize;
    fn total_frames(&self, styled: &[StyledChar]) -> usize; // default: len + cooldown_frames
    fn render_frame(&self, styled: &[StyledChar], frame: usize, buf: &mut String);
}
```

`main` owns the frame loop and calls into the trait. The loop clears `buf` before each `render_frame` call; animations only append. A shared `cooldown_color(age, cooldown_frames, gradient)` helper in `anim/mod.rs` maps cooldown age to a 256-color index.

### Adding a new animation

1. Create `src/anim/foo.rs` with struct implementing `Animation`
2. Add `mod foo;` to `src/anim/mod.rs`
3. Add match arm in `resolve()`

No changes to `main.rs` or other animation files.

### Animations

Available animations (`zest ANIMATION [COLOR]`):

| Name | Description | Colors |
|---|---|---|
| `sprout` | Green cooling gradient sweep (default) | — |
| `flames` | Fire sweep with flickering dot-matrix characters | `orange` (default), `blue`, `green`, `purple`, `pink` |
| `matrix` | Random ASCII decodes into correct chars | `green` (default), `blue`, `red`, `orange`, `purple`, `pink` |
| `scan` | CRT phosphor sweep, brief white afterglow | — |
| `lightning` | Instant reveal with bright yellow flash band sweeping left-to-right | — |

### Sprout animation (`anim/sprout.rs`)

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

Characters cool down over `COOLDOWN_FRAMES` frames through the selected color gradient using ANSI 256-color mode. Once fully cooled, each character snaps to its actual prompt color.

The `gradient_for(color)` function maps an optional color name to the appropriate `GRADIENT_*` constant. `Flames` holds the resolved gradient as a field.

| Constant | Purpose |
|---|---|
| `FRAME_DELAY_MS` | Speed of animation |
| `COOLDOWN_FRAMES` | Length of the fire wake behind the leading edge |
| `GRADIENT` | Orange default: `#ffff00` → `#870000` |
| `GRADIENT_BLUE` | White-blue → dark navy |
| `GRADIENT_GREEN` | Bright green → dark green |
| `GRADIENT_PURPLE` | Pink-magenta → dark violet |
| `GRADIENT_PINK` | Solid hot pink (`#ff0087`) |
| `FLAME_CHARS` | Braille/block chars used during the fire phase |

### Matrix animation (`anim/matrix.rs`)

Characters sweep in from the left, one per frame, starting at frame 2. During cooldown, each position shows a random ASCII character chosen via splitmix64-style hash. Once fully cooled, characters snap to their actual prompt color.

`Matrix` holds a gradient field (same pattern as `Flames`). The `gradient_for(color)` function maps an optional color name to the appropriate `GRADIENT_*` constant.

| Constant | Purpose |
|---|---|
| `COOLDOWN_FRAMES` | Length of the scramble wake |
| `GRADIENT` | Green default: `#87ff00` → `#008700` |
| `GRADIENT_BLUE` | White-blue → dark navy |
| `GRADIENT_RED` | Bright red → dark red |
| `GRADIENT_ORANGE` | Orange-yellow → dark red |
| `GRADIENT_PURPLE` | Pink-magenta → dark violet |
| `GRADIENT_PINK` | Solid hot pink (`#ff0087`) |
| `MATRIX_CHARS` | ASCII characters used during the scramble phase |

### Lightning animation (`anim/lightning.rs`)

The entire prompt is shown at its real colors from frame 1 — no reveal sweep. A **flash band** of 9 characters sweeps left-to-right at half speed (one character position every two frames), giving it a slow, dramatic feel.

Each character in the band is rendered with both a foreground and background color based on its distance from the band center:

| Distance | Foreground | Background |
|---|---|---|
| 0 (core) | 231 `#ffffff` white | 100 `#878700` dark yellow |
| 1 | 226 `#ffff00` bright yellow | 58 `#5f5f00` dark olive |
| 2 | 220 `#ffd700` gold | 238 `#444444` dark grey |
| 3 | 214 `#ffaf00` orange-gold | 237 `#3a3a3a` darker grey |
| 4 (edge) | 178 `#d7af00` dark gold | 236 `#303030` near-black |

Characters outside the band always show their actual prompt color. After the band exits the right edge, all characters are at their real colors.

`total_frames` is overridden to `2 * (n + BAND_HALF) + 2` to account for the half-speed movement.

| Constant | Purpose |
|---|---|
| `BAND_HALF` | Half-width of the flash band (4 → 9 chars total) |
| `FLASH_FG` | Foreground 256-color gradient from center outward |
| `FLASH_BG` | Background 256-color gradient from center outward |

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
