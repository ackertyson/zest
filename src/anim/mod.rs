mod flames;
mod sprout;
mod shine;
mod matrix;
mod scan;

use std::fmt::Write;

use crate::style::{color256, StyledChar};


pub const DEFAULT: &str = "flames";

pub const LIST: &[(&str, &str)] = &[
    ("sprout",  "Green cooling gradient sweep"),
    ("flames",      "Fire sweep with flickering dot-matrix characters"),
    ("matrix",      "Random ASCII decodes into correct chars"),
    ("scan",        "CRT phosphor sweep, brief white afterglow"),
    ("shine",   "Instant reveal with bright yellow flash band sweep"),
];

pub const COLORS: &[(&str, &[&str])] = &[
    ("sprout",  &["green", "orange", "blue", "purple", "pink"]),
    ("flames",  &["orange", "blue", "green", "purple", "pink"]),
    ("matrix",  &["green", "blue", "red", "orange", "purple", "pink"]),
    ("scan",    &["white", "blue", "green", "orange", "purple", "pink", "red"]),
    ("shine",   &["yellow", "blue", "green", "orange", "purple", "pink", "red"]),
];

pub trait Animation {
    fn cooldown_frames(&self) -> usize;
    fn total_frames(&self, styled: &[StyledChar]) -> usize {
        styled.len() + self.cooldown_frames()
    }
    fn render_frame(&self, styled: &[StyledChar], frame: usize, buf: &mut String);
}

// ── Shared color gradients (256-color indices) ──────────────────────────────

// bright orange-yellow → orange → red-orange → red → dark red
//   226=#ffff00  220=#ffd700  214=#ffaf00  208=#ff8700
//   202=#ff5f00  196=#ff0000  160=#d70000   88=#870000
pub const GRADIENT_ORANGE: &[u8] = &[226, 220, 214, 208, 202, 196, 160, 88];

// white-blue → cyan → sky-blue → blue → dark navy
//   231=#ffffff  195=#d7ffff  159=#afffff  123=#87ffff   87=#5fffff   51=#00ffff
//    45=#00d7ff   39=#00afff   33=#0087ff   27=#005fff   21=#0000ff   18=#000087   17=#00005f
pub const GRADIENT_BLUE: &[u8] = &[231, 195, 159, 123, 87, 51, 45, 39, 33, 27, 21, 18, 17];

// bright green → green → dark green
//   157=#afffaf  120=#87ff87   83=#5fff5f   46=#00ff00
//    40=#00d700   34=#00af00   28=#008700   22=#005f00
pub const GRADIENT_GREEN: &[u8] = &[157, 120, 83, 46, 40, 34, 28, 22];

// pink/magenta → purple → dark purple
//   219=#ffafff  213=#ff87ff  207=#ff5fff  201=#ff00ff
//   165=#d700ff  129=#af00ff   93=#8700ff   57=#5f00ff   55=#5f00af
pub const GRADIENT_PURPLE: &[u8] = &[219, 213, 207, 201, 165, 129, 93, 57, 55];

// solid hot pink — single color: 198=#ff0087
pub const GRADIENT_PINK: &[u8] = &[198];

// bright red → dark red
//   196=#ff0000  160=#d70000  124=#af0000   88=#870000   52=#5f0000
pub const GRADIENT_RED: &[u8] = &[196, 160, 124, 88, 52];

pub fn cooldown_color(age: usize, cooldown_frames: usize, gradient: &[u8]) -> u8 {
    let steps = gradient.len() - 1;
    let idx = ((age * steps) / cooldown_frames).min(steps);
    gradient[idx]
}

/// Splitmix64-style hash of position + frame — used by animations for deterministic
/// per-cell randomness that avalanches all input bits.
pub(super) fn hash(pos: usize, frame: usize) -> usize {
    let mut h = pos.wrapping_add(frame.wrapping_mul(0x9e3779b97f4a7c15));
    h = (h ^ (h >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94d049bb133111eb);
    h ^ (h >> 31)
}

pub(super) fn revealed(frame: usize, n: usize) -> usize {
    if frame >= 2 { (frame - 2).min(n) } else { 0 }
}

pub(super) fn last_content(styled: &[StyledChar]) -> usize {
    styled.iter().rposition(|sc| !sc.ch.is_whitespace()).unwrap_or(styled.len())
}

pub(super) fn has_leading(frame: usize, revealed: usize, n: usize, last_content: usize) -> bool {
    frame >= 2 && revealed < n && revealed < last_content
}

/// Shared left-to-right sweep renderer used by sprout, flames, matrix, and scan.
///
/// - `cooldown_char`: given (position, frame, &StyledChar), returns the character to display
///   during cooldown. Return `sc.ch` for animations that show the real character, or an effect
///   character (flame/matrix glyph) for texture animations.
/// - `snap_trailing`: if true, characters at or past `last_content` always snap to their real
///   color immediately (used by flames/matrix to avoid effects on the trailing chevron).
/// - `render_leading`: renders the leading-edge character into `buf`, given (frame, revealed
///   index, styled slice, buf). Called only when a leading edge should be shown.
pub(super) fn render_sweep<F, L>(
    styled: &[StyledChar],
    frame: usize,
    buf: &mut String,
    cooldown_frames: usize,
    gradient: &[u8],
    bg_gradient: Option<&[u8]>,
    snap_trailing: bool,
    cooldown_char: F,
    render_leading: L,
) where
    F: Fn(usize, usize, &StyledChar) -> char,
    L: Fn(usize, usize, &[StyledChar], &mut String),
{
    let n = styled.len();
    let rev = revealed(frame, n);
    let lc = last_content(styled);
    let has_lead = has_leading(frame, rev, n, lc);

    for (i, sc) in styled[..rev].iter().enumerate() {
        let age = frame.saturating_sub(i + 3);
        if age >= cooldown_frames || (snap_trailing && i >= lc) {
            buf.push_str("\x1b[0m");
            buf.push_str(&sc.color_prefix);
            buf.push(sc.ch);
        } else {
            color256(buf, cooldown_color(age, cooldown_frames, gradient));
            if let Some(bg) = bg_gradient {
                if age < bg.len() {
                    write!(buf, "\x1b[48;5;{}m", bg[age]).unwrap();
                } else {
                    buf.push_str("\x1b[49m");
                }
            }
            buf.push(cooldown_char(i, frame, sc));
        }
    }

    if has_lead {
        if bg_gradient.is_some() {
            buf.push_str("\x1b[49m");
        }
        render_leading(frame, rev, styled, buf);
    }

    buf.push_str("\x1b[0m");
}

fn leak(g: &[u8]) -> &'static [u8] {
    Box::leak(g.to_vec().into_boxed_slice())
}

pub fn resolve(name: &str, color: Option<&str>, custom_fg: Option<&[u8]>, custom_bg: Option<&[u8]>, flip_rate: usize) -> Option<Box<dyn Animation>> {
    match name {
        "sprout" => {
            let gradient: &'static [u8] = if let Some(g) = custom_fg {
                leak(g)
            } else {
                sprout::gradient_for(color)?
            };
            let bg_gradient: Option<&'static [u8]> = custom_bg.map(leak);
            Some(Box::new(sprout::Sprout { gradient, bg_gradient }))
        }
        "flames" => {
            let gradient: &'static [u8] = if let Some(g) = custom_fg {
                leak(g)
            } else {
                flames::gradient_for(color)?
            };
            let bg_gradient: Option<&'static [u8]> = custom_bg.map(leak);
            Some(Box::new(flames::Flames { gradient, bg_gradient, glyph_frames: flip_rate }))
        }
        "matrix" => {
            let gradient: &'static [u8] = if let Some(g) = custom_fg {
                leak(g)
            } else {
                matrix::gradient_for(color)?
            };
            let bg_gradient: Option<&'static [u8]> = custom_bg.map(leak);
            Some(Box::new(matrix::Matrix { gradient, bg_gradient, glyph_frames: flip_rate }))
        }
        "scan" => {
            let gradient: &'static [u8] = if let Some(g) = custom_fg {
                leak(g)
            } else {
                scan::gradient_for(color)?
            };
            let bg_gradient: Option<&'static [u8]> = custom_bg.map(leak);
            Some(Box::new(scan::Scan { gradient, bg_gradient }))
        }
        "shine" => {
            let (named_fg, named_bg) = shine::gradient_for(color)?;
            let flash_fg: &'static [u8] = if let Some(g) = custom_fg { leak(g) } else { named_fg };
            let flash_bg: Option<&'static [u8]> = if let Some(g) = custom_bg {
                Some(leak(g))
            } else if custom_fg.is_none() {
                Some(named_bg)
            } else {
                None
            };
            Some(Box::new(shine::Shine { flash_fg, flash_bg }))
        }
        _ => None,
    }
}
