use crate::style::{StyledChar, color256};

use super::Animation;

const COOLDOWN_FRAMES: usize = 14;

const FLAME_CHARS: &[char] = &[
    '⣀', '⠰', '⠸', '⠼', '⣤', '⣶', '⣿', // original
    '⠶', '⠷', '⠾', '⠿', '⡶', '⡷', '⣖', '⣞', '⣝', '⣛', '⣚', // mid-density
];

use super::{GRADIENT_BLUE, GRADIENT_GREEN, GRADIENT_ORANGE, GRADIENT_PINK, GRADIENT_PURPLE};

pub struct Flames {
    pub(super) gradient: &'static [u8],
    pub(super) bg_gradient: Option<&'static [u8]>,
    pub(super) glyph_frames: usize,
}

pub fn gradient_for(color: Option<&str>) -> Option<&'static [u8]> {
    match color {
        None | Some("orange") => Some(GRADIENT_ORANGE),
        Some("blue") => Some(GRADIENT_BLUE),
        Some("green") => Some(GRADIENT_GREEN),
        Some("purple") => Some(GRADIENT_PURPLE),
        Some("pink") => Some(GRADIENT_PINK),
        _ => None,
    }
}

fn flame_char(pos: usize, frame: usize) -> char {
    FLAME_CHARS[super::hash(pos, frame) % FLAME_CHARS.len()]
}

/// Sinusoidal wave heat — gives organic flowing color variation across the cooldown wake.
/// Coefficient 0.22 tuned for zest's 10ms frame rate.
fn wave_heat(pos: usize, frame: usize) -> f32 {
    let t = frame as f32 * 0.22;
    let p = pos as f32 * 0.7;
    let v = (t + p).sin() * 0.38 + (t * 1.9 - p * 0.5).sin() * 0.22 + 0.5;
    v.clamp(0.0, 1.0)
}

fn wave_color(pos: usize, frame: usize, gradient: &[u8]) -> u8 {
    let heat = wave_heat(pos, frame);
    let idx = ((1.0 - heat) * (gradient.len() - 1) as f32).round() as usize;
    gradient[idx.min(gradient.len() - 1)]
}

impl Animation for Flames {
    fn cooldown_frames(&self) -> usize {
        COOLDOWN_FRAMES
    }

    fn render_frame(&self, styled: &[StyledChar], frame: usize, buf: &mut String) {
        let glyph_frames = self.glyph_frames;
        super::render_sweep(
            styled,
            frame,
            buf,
            COOLDOWN_FRAMES,
            self.gradient,
            self.bg_gradient,
            true,
            |pos, _age, frame, gradient| wave_color(pos, frame, gradient),
            |pos, frame, _sc| flame_char(pos, frame / glyph_frames),
            |_frame, revealed, _styled, buf| {
                color256(buf, self.gradient[0]);
                buf.push(flame_char(revealed, frame / glyph_frames));
            },
        );
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
        Flames {
            gradient: GRADIENT_ORANGE,
            bg_gradient: None,
            glyph_frames: 6,
        }
        .render_frame(&styled, 1, &mut buf);
        assert!(!buf.contains('a'));
    }

    #[test]
    fn leading_edge_present_at_frame_2() {
        let styled = parse_styled("ab");
        let mut buf = String::new();
        Flames {
            gradient: GRADIENT_ORANGE,
            bg_gradient: None,
            glyph_frames: 6,
        }
        .render_frame(&styled, 2, &mut buf);
        assert!(buf.len() > "\x1b[0m".len());
    }

    #[test]
    fn chars_snap_after_cooldown() {
        let styled = parse_styled("a");
        let mut buf = String::new();
        let snap_frame = 3 + COOLDOWN_FRAMES;
        Flames {
            gradient: GRADIENT_ORANGE,
            bg_gradient: None,
            glyph_frames: 6,
        }
        .render_frame(&styled, snap_frame, &mut buf);
        assert!(buf.contains('a'));
    }
}
