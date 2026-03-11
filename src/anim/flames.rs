use std::fmt::Write;

use crate::style::{color256, StyledChar};

use super::Animation;

const FRAME_DELAY_MS: u64 = 10;
const COOLDOWN_FRAMES: usize = 14;

// Braille and block chars for a dot-matrix / fire-texture feel
const FLAME_CHARS: &[char] = &[
    '⣀', '⠠', '⠰', '⠸', '⠼', '⣤', '⣶', '⣿',
];

// 256-color gradient: bright orange-yellow → orange → red-orange → red → dark red
//   226=#ffff00  220=#ffd700  214=#ffaf00  208=#ff8700
//   202=#ff5f00  196=#ff0000  160=#d70000   88=#870000
const GRADIENT: &[u8] = &[226, 220, 214, 208, 202, 196, 160, 88];

pub struct Flames;

/// Deterministic pseudo-random char selection based on position and frame.
/// Uses a splitmix64-style finalizer so all output bits avalanche from all
/// input bits — avoids the low-bit rigidity of a simple multiply-mod.
fn flame_char(pos: usize, frame: usize) -> char {
    let mut h = pos.wrapping_add(frame.wrapping_mul(0x9e3779b97f4a7c15));
    h = (h ^ (h >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94d049bb133111eb);
    h ^= h >> 31;
    FLAME_CHARS[h % FLAME_CHARS.len()]
}

fn cooldown_color(age: usize) -> u8 {
    let steps = GRADIENT.len() - 1;
    let idx = ((age * steps) / COOLDOWN_FRAMES).min(steps);
    GRADIENT[idx]
}

impl Animation for Flames {
    fn total_frames(&self, n: usize) -> usize {
        n + COOLDOWN_FRAMES
    }

    fn frame_delay_ms(&self) -> u64 {
        FRAME_DELAY_MS
    }

    fn render_frame(&self, styled: &[StyledChar], n: usize, frame: usize, buf: &mut String) {
        let revealed = if frame >= 2 { (frame - 2).min(n) } else { 0 };
        let has_leading = frame >= 2 && revealed < n;

        // Revealed chars: show flickering dot-matrix during cooldown, then snap to real color.
        for (i, sc) in styled[..revealed].iter().enumerate() {
            let age = frame.saturating_sub(i + 3);
            if age >= COOLDOWN_FRAMES {
                buf.push_str("\x1b[0m");
                buf.push_str(&sc.color_prefix);
                buf.push(sc.ch);
            } else {
                color256(buf, cooldown_color(age));
                buf.push(flame_char(i, frame));
            }
        }

        // Leading edge: hottest color, flickering dot-matrix char
        if has_leading {
            color256(buf, GRADIENT[0]);
            write!(buf, "{}", flame_char(revealed, frame)).unwrap();
        }

        buf.push_str("\x1b[0m");
    }
}

#[cfg(test)]
mod tests {
    use crate::style::parse_styled;

    use super::*;

    #[test]
    fn no_output_before_animation_starts() {
        let styled = parse_styled("abc");
        let mut buf = String::new();
        Flames.render_frame(&styled, 3, 1, &mut buf);
        assert!(!buf.contains('a'));
    }

    #[test]
    fn leading_edge_present_at_frame_2() {
        let styled = parse_styled("ab");
        let mut buf = String::new();
        Flames.render_frame(&styled, 2, 2, &mut buf);
        // Frame 2: 0 revealed, 1 leading edge char — buf should be non-empty
        assert!(buf.len() > "\x1b[0m".len());
    }

    #[test]
    fn chars_snap_after_cooldown() {
        let styled = parse_styled("a");
        let mut buf = String::new();
        // age for char 0 = frame - 3; fully cooled when age >= COOLDOWN_FRAMES
        let snap_frame = 3 + COOLDOWN_FRAMES;
        Flames.render_frame(&styled, 1, snap_frame, &mut buf);
        assert!(buf.contains('a'));
    }
}
