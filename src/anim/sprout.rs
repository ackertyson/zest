use std::fmt::Write;

use crate::style::StyledChar;

use super::Animation;
use super::{GRADIENT_BLUE, GRADIENT_GREEN, GRADIENT_ORANGE, GRADIENT_PINK, GRADIENT_PURPLE};

const SPINNERS: [char; 4] = ['-', '\\', '|', '/'];
const COOLDOWN_FRAMES: usize = 12;

pub struct Sprout {
    pub(super) gradient: &'static [u8],
    pub(super) bg_gradient: Option<&'static [u8]>,
}

pub fn gradient_for(color: Option<&str>) -> Option<&'static [u8]> {
    match color {
        None | Some("green") => Some(GRADIENT_GREEN),
        Some("orange") => Some(GRADIENT_ORANGE),
        Some("blue") => Some(GRADIENT_BLUE),
        Some("purple") => Some(GRADIENT_PURPLE),
        Some("pink") => Some(GRADIENT_PINK),
        _ => None,
    }
}

impl Animation for Sprout {
    fn cooldown_frames(&self) -> usize {
        COOLDOWN_FRAMES
    }

    fn render_frame(&self, styled: &[StyledChar], frame: usize, buf: &mut String) {
        super::render_sweep(
            styled,
            frame,
            buf,
            COOLDOWN_FRAMES,
            self.gradient,
            self.bg_gradient,
            false,
            |_pos, age, _frame, gradient| super::cooldown_color(age, COOLDOWN_FRAMES, gradient),
            |_pos, _frame, sc| sc.ch,
            |frame, _revealed, _styled, buf| {
                write!(buf, "\x1b[97m{}", SPINNERS[(frame - 1) % SPINNERS.len()]).unwrap();
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::style::parse_styled;

    use super::*;

    #[test]
    fn render_frame_reveals_first_char_at_frame_1() {
        let styled = parse_styled("abc");
        let mut buf = String::new();
        Sprout {
            gradient: GRADIENT_GREEN,
            bg_gradient: None,
        }
        .render_frame(&styled, 1, &mut buf);
        // Frame 1: first char revealed with spinner leading edge
        assert!(buf.len() > "\x1b[0m".len());
    }

    #[test]
    fn render_frame_reveals_chars() {
        let styled = parse_styled("ab");
        let mut buf = String::new();
        Sprout {
            gradient: GRADIENT_GREEN,
            bg_gradient: None,
        }
        .render_frame(&styled, 1, &mut buf);
        // Frame 1: 1 char revealed
        assert!(buf.contains('a'));
    }
}
