# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Core Principles

1. **Ultra-fast startup** — Every millisecond of latency before the animation begins is felt on every prompt draw. Minimize dependencies, pre-allocate buffers, and optimize CPU/memory on the hot path. Animation *duration* is a deliberate aesthetic choice (balancing speed with visual impact), not a performance target — don't shorten animations to "go faster."
2. **Effortless fish/zsh integration** — Pipe to `zest` and go. Existing prompt configs need minimal adjustment.
3. **Elegant, streamlined, idiomatic Rust** — Clean trait-based architecture, minimal deps, no unnecessary abstractions.
4. **Flexible composability** — Animation patterns and color gradients mix freely.

## Commands

```bash
cargo fmt               # format code (run after changes)
cargo clippy            # lint (run after changes, fix any warnings)
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

# Custom 256-color gradient (overrides named color)
printf '\x1b[36m~/projects\x1b[0m \x1b[96m❯ \x1b[0m' | cargo run -- sprout --gradient 226,220,214,88

# Custom FG + BG gradient (colon-separated)
printf '\x1b[36m~/projects\x1b[0m \x1b[96m❯ \x1b[0m' | cargo run -- sprout --gradient 226,220:52,88

# Plain text fallback (no pipe)
cargo run -- "hello world"

# Help
cargo run -- --help

# Release a new version (bumps Cargo.toml, commits, tags, pushes)
./build/tag.sh 0.2.0
```

### Releasing

`build/tag.sh <version>` handles the full release flow: updates `Cargo.toml` version, commits, creates a `v<version>` git tag, and pushes (after confirmation). Requires clean working tree on `master`, in sync with `origin/master`.

Pushing a `v*` tag triggers `.github/workflows/release.yml`, which runs tests, builds release binaries for macOS (aarch64 + x86_64), and creates a GitHub Release with tarballs and checksums.

Rust toolchain version is pinned in `rust-toolchain.toml`.

## Architecture

Rust CLI that reads a colorized prompt from **stdin** (or falls back to CLI args if stdin is a TTY) and animates it into view in the terminal.

### File layout

```
src/
  main.rs              -- CLI parsing, input reading, animation loop
  style.rs             -- StyledChar, parse_styled(), color256() (shared infra)
  shell.rs             -- wrap_ansi_for_zsh(), is_zsh() (shell-specific logic)
  anim/
    mod.rs             -- Animation trait, resolve() dispatch, cooldown_color(), shared GRADIENT_* constants
    sprout.rs          -- "sprout" animation
    flames.rs          -- "flames" animation with color variants (orange/blue/green/purple/pink)
    matrix.rs          -- "matrix" animation
    scan.rs            -- "scan" animation
    shine.rs       -- "shine" animation
```

### CLI

- `zest [ANIMATION [COLOR]]` — optional positional args select animation and color variant (default: `flames`)
- `--gradient <fg[:bg]>` — comma-separated 256-color FG indices with an optional `:bg,...` list for background colors; each side is independent and arbitrary-length; bad/empty sides silently fall back to default
- `--flip-rate <n>` — glyph change rate for flames/matrix (1–20, default 4); controls how rapidly scramble glyphs cycle, tuning the animation from frenetic (1) to deliberate (20); only affects those two animations
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

Shared 256-color gradients live in `anim/mod.rs` and are used by sprout, flames, and matrix via their per-module `gradient_for()` functions:

| Constant | Description |
|---|---|
| `GRADIENT_ORANGE` | `#ffff00` → `#870000` (flames default) |
| `GRADIENT_BLUE` | White-blue → dark navy |
| `GRADIENT_GREEN` | White-green → dark green (sprout/matrix default) |
| `GRADIENT_PURPLE` | Pink-magenta → dark violet |
| `GRADIENT_PINK` | Solid hot pink (`#ff0087`) |
| `GRADIENT_RED` | Bright red → dark red |

### Adding a new animation

1. Create `src/anim/foo.rs` with struct implementing `Animation`
2. Add `mod foo;` to `src/anim/mod.rs`
3. Add match arm in `resolve()`

No changes to `main.rs` or other animation files.

### Animations

Available animations (`zest ANIMATION [COLOR]`):

| Name | Description | Colors |
|---|---|---|
| `sprout` | Cooling gradient sweep | `green` (default), `orange`, `blue`, `purple`, `pink` |
| `flames` | Fire sweep with flickering dot-matrix characters (default) | `orange` (default), `blue`, `green`, `purple`, `pink` |
| `matrix` | Random ASCII decodes into correct chars | `green` (default), `blue`, `red`, `orange`, `purple`, `pink` |
| `scan` | CRT phosphor sweep, brief white afterglow | `white` (default), `blue`, `green`, `orange`, `purple`, `pink`, `red` |
| `shine` | Instant reveal with bright flash band sweeping left-to-right | `yellow` (default), `blue`, `green`, `orange`, `purple`, `pink`, `red` |

### Sprout animation (`anim/sprout.rs`)

Characters sweep in from the left, one per frame, starting at frame 2. A single **spinner character** (`-\|/` cycling) advances rightward one position per frame, acting as the leading edge.

Characters behind the spinner "cool down" over `COOLDOWN_FRAMES` frames:
- **Cooling phase** (age < COOLDOWN_FRAMES): gradient from hot color to dark, using the selected color variant
- **Fully cooled** (age >= COOLDOWN_FRAMES): snaps to the character's **actual prompt color** from the original ANSI input

After the animation loop, the **exact original input** is written as the final frame (pixel-perfect reproduction).

Uses **ANSI 256-color mode** (`\x1b[38;5;Nm`) for the cooling gradient. The `gradient_for(color)` function maps an optional color name to the appropriate shared gradient constant from `anim/mod.rs`. The spinner uses standard 16-color bright white (`\x1b[97m`). Final resting colors come from the original prompt's ANSI sequences. An optional `bg_gradient` field applies per-character background colors during cooldown, indexed directly by `age` (independent of `COOLDOWN_FRAMES`).

| Constant | Purpose |
|---|---|
| `FRAME_DELAY_MS` | Speed of animation |
| `COOLDOWN_FRAMES` | Length of the wake behind the spinner |

### Flames animation (`anim/flames.rs`)

Characters sweep in from the left, one per frame, starting at frame 2. The leading edge and cooling characters are rendered as Braille/block dot-matrix chars (`FLAME_CHARS`) chosen deterministically by position and frame via a splitmix64-style hash, giving a flickering fire texture.

Characters cool down over `COOLDOWN_FRAMES` frames through the selected color gradient using ANSI 256-color mode. Once fully cooled, each character snaps to its actual prompt color.

The `gradient_for(color)` function maps an optional color name to the appropriate shared `GRADIENT_*` constant from `anim/mod.rs`. `Flames` holds the resolved gradient and an optional `bg_gradient` field; BG colors are applied during cooldown indexed directly by `age`.

| Constant/Field | Purpose |
|---|---|
| `FRAME_DELAY_MS` | Speed of animation |
| `COOLDOWN_FRAMES` | Length of the fire wake behind the leading edge |
| `FLAME_CHARS` | Braille/block chars used during the fire phase |
| `glyph_frames` (field) | Frames each flame glyph holds before changing; set from `--flip-rate` (default 4) |

### Matrix animation (`anim/matrix.rs`)

All characters appear as scrambled ASCII glyphs at frame 2. Characters then resolve to their actual prompt in a **random order** (one per frame), each cooling through the color gradient over `COOLDOWN_FRAMES` before snapping to its real color. The resolve order is a deterministic Fisher-Yates permutation seeded via `hash()`, stored lazily in a `OnceCell<Vec<usize>>` mapping position → trigger step.

`Matrix` holds a gradient field and an optional `bg_gradient` field (same pattern as `Flames`). The `gradient_for(color)` function maps an optional color name to the appropriate shared `GRADIENT_*` constant from `anim/mod.rs`.

| Constant/Field | Purpose |
|---|---|
| `COOLDOWN_FRAMES` | Length of the cooldown gradient per character |
| `MATRIX_CHARS` | ASCII characters used during the scramble phase |
| `glyph_frames` (field) | Frames each scramble glyph holds before changing; set from `--flip-rate` (default 4) |
| `trigger` (field) | `OnceCell<Vec<usize>>` — lazily-built permutation mapping position → reveal step |

### Shine animation (`anim/shine.rs`)

The entire prompt is shown at its real colors from frame 1 — no reveal sweep. A **flash band** sweeps left-to-right, one character position per frame (same rate as other sweep animations).

Each character in the band is rendered with FG and/or BG color based on its distance from the band center. A character at distance `d` is in the FG band if `d < flash_fg.len()`, and in the BG band if `flash_bg` is Some and `d < flash_bg.len()`. The two are independent and arbitrary-length. Characters outside both bands show their actual prompt color.

The default yellow named-color band is 9 characters wide (distances 0–4):

| Distance | Foreground | Background |
|---|---|---|
| 0 (core) | 231 `#ffffff` white | 100 `#878700` dark yellow |
| 1 | 226 `#ffff00` bright yellow | 58 `#5f5f00` dark olive |
| 2 | 220 `#ffd700` gold | 238 `#444444` dark grey |
| 3 | 214 `#ffaf00` orange-gold | 237 `#3a3a3a` darker grey |
| 4 (edge) | 178 `#d7af00` dark gold | 236 `#303030` near-black |

`flash_bg` is `Option<&'static [u8]>` — `None` means no background (e.g. when a custom FG is given without a custom BG). `BAND_HALF` is used in the `total_frames` calculation to ensure the band fully exits the visible area.

| Constant | Purpose |
|---|---|
| `BAND_HALF` | Used only for `total_frames` calculation (keeps default-sized bands fully visible) |
| `FLASH_FG` | Default (yellow) foreground 256-color gradient from center outward |
| `FLASH_BG` | Default (yellow) background 256-color gradient from center outward |
| `FLASH_FG_*` / `FLASH_BG_*` | Per-color FG/BG band gradient pairs (blue, green, orange, purple, pink, red) |

### Fish shell integration

```fish
function fish_prompt
    set -l last_pipestatus $pipestatus
    set -lx __fish_last_status $status
    set -l _zest cat
    command -q zest; and set _zest zest
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
    end | $_zest
end
```
