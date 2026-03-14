use crate::style::{color256, StyledChar};

use super::Animation;

const COOLDOWN_FRAMES: usize = 14;

// Braille and block chars for a dot-matrix / fire-texture feel
const FLAME_CHARS: &[char] = &[
    '⣀', '⠠', '⠰', '⠸', '⠼', '⣤', '⣶', '⣿',
];

use super::{GRADIENT_ORANGE, GRADIENT_BLUE, GRADIENT_GREEN, GRADIENT_PURPLE, GRADIENT_PINK};

pub struct Flames {
    pub(super) gradient: &'static [u8],
}

pub fn gradient_for(color: Option<&str>) -> Option<&'static [u8]> {
    match color {
        None | Some("orange") => Some(GRADIENT_ORANGE),
        Some("blue")          => Some(GRADIENT_BLUE),
        Some("green")         => Some(GRADIENT_GREEN),
        Some("purple")        => Some(GRADIENT_PURPLE),
        Some("pink")          => Some(GRADIENT_PINK),
        _                     => None,
    }
}

fn flame_char(pos: usize, frame: usize) -> char {
    FLAME_CHARS[super::hash(pos, frame) % FLAME_CHARS.len()]
}

impl Animation for Flames {
    fn cooldown_frames(&self) -> usize { COOLDOWN_FRAMES }

    fn render_frame(&self, styled: &[StyledChar], frame: usize, buf: &mut String) {
        let gradient = self.gradient;
        super::render_sweep(
            styled, frame, buf,
            COOLDOWN_FRAMES, gradient,
            true,
            |pos, frame, _sc| flame_char(pos, frame),
            |_frame, revealed, _styled, buf| {
                color256(buf, gradient[0]);
                buf.push(flame_char(revealed, frame));
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
        Flames { gradient: GRADIENT_ORANGE }.render_frame(&styled, 1, &mut buf);
        assert!(!buf.contains('a'));
    }

    #[test]
    fn leading_edge_present_at_frame_2() {
        let styled = parse_styled("ab");
        let mut buf = String::new();
        Flames { gradient: GRADIENT_ORANGE }.render_frame(&styled, 2, &mut buf);
        // Frame 2: 0 revealed, 1 leading edge char — buf should be non-empty
        assert!(buf.len() > "\x1b[0m".len());
    }

    #[test]
    fn chars_snap_after_cooldown() {
        let styled = parse_styled("a");
        let mut buf = String::new();
        // age for char 0 = frame - 3; fully cooled when age >= COOLDOWN_FRAMES
        let snap_frame = 3 + COOLDOWN_FRAMES;
        Flames { gradient: GRADIENT_ORANGE }.render_frame(&styled, snap_frame, &mut buf);
        assert!(buf.contains('a'));
    }
}
