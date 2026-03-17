use crate::style::{StyledChar, color256};

use super::Animation;
use super::{GRADIENT_BLUE, GRADIENT_ORANGE, GRADIENT_PINK, GRADIENT_PURPLE, GRADIENT_RED};

const COOLDOWN_FRAMES: usize = 12;

const MATRIX_CHARS: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*+=<>?/";

// Matrix-specific green: brighter starting point than the shared green
//   118=#87ff00   82=#5fff00   46=#00ff00   40=#00d700   34=#00af00   28=#008700
const GRADIENT: &[u8] = &[118, 82, 46, 40, 34, 28];

pub struct Matrix {
    pub(super) gradient: &'static [u8],
    pub(super) bg_gradient: Option<&'static [u8]>,
    pub(super) glyph_frames: usize,
}

pub fn gradient_for(color: Option<&str>) -> Option<&'static [u8]> {
    match color {
        None | Some("green") => Some(GRADIENT),
        Some("blue") => Some(GRADIENT_BLUE),
        Some("red") => Some(GRADIENT_RED),
        Some("orange") => Some(GRADIENT_ORANGE),
        Some("purple") => Some(GRADIENT_PURPLE),
        Some("pink") => Some(GRADIENT_PINK),
        _ => None,
    }
}

fn matrix_char(pos: usize, frame: usize) -> char {
    MATRIX_CHARS[super::hash(pos, frame) % MATRIX_CHARS.len()] as char
}

impl Animation for Matrix {
    fn cooldown_frames(&self) -> usize {
        COOLDOWN_FRAMES
    }

    fn render_frame(&self, styled: &[StyledChar], frame: usize, buf: &mut String) {
        let gradient = self.gradient;
        let glyph_frames = self.glyph_frames;
        super::render_sweep(
            styled,
            frame,
            buf,
            COOLDOWN_FRAMES,
            gradient,
            self.bg_gradient,
            true,
            |_pos, age, _frame, gradient| super::cooldown_color(age, COOLDOWN_FRAMES, gradient),
            |pos, frame, _sc| matrix_char(pos, frame / glyph_frames),
            |_frame, revealed, _styled, buf| {
                color256(buf, gradient[0]);
                buf.push(matrix_char(revealed, frame / glyph_frames));
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
        Matrix {
            gradient: GRADIENT,
            bg_gradient: None,
            glyph_frames: 6,
        }
        .render_frame(&styled, 1, &mut buf);
        assert!(!buf.contains('a'));
    }

    #[test]
    fn chars_snap_after_cooldown() {
        let styled = parse_styled("a");
        let mut buf = String::new();
        let snap_frame = 3 + COOLDOWN_FRAMES;
        Matrix {
            gradient: GRADIENT,
            bg_gradient: None,
            glyph_frames: 6,
        }
        .render_frame(&styled, snap_frame, &mut buf);
        assert!(buf.contains('a'));
    }
}
