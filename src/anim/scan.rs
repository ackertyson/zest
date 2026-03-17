use crate::style::StyledChar;

use super::Animation;
use super::{
    GRADIENT_BLUE, GRADIENT_GREEN, GRADIENT_ORANGE, GRADIENT_PINK, GRADIENT_PURPLE, GRADIENT_RED,
};

const COOLDOWN_FRAMES: usize = 4;

// 256-color gradient: white → fading white
//   231=#ffffff  195=#d7ffff  189=#d7d7ff  183=#d7afff
const GRADIENT: &[u8] = &[231, 195, 189, 183];

pub struct Scan {
    pub(super) gradient: &'static [u8],
    pub(super) bg_gradient: Option<&'static [u8]>,
}

pub fn gradient_for(color: Option<&str>) -> Option<&'static [u8]> {
    match color {
        None | Some("white") => Some(GRADIENT),
        Some("blue") => Some(GRADIENT_BLUE),
        Some("green") => Some(GRADIENT_GREEN),
        Some("orange") => Some(GRADIENT_ORANGE),
        Some("purple") => Some(GRADIENT_PURPLE),
        Some("pink") => Some(GRADIENT_PINK),
        Some("red") => Some(GRADIENT_RED),
        _ => None,
    }
}

impl Animation for Scan {
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
            |_frame, revealed, styled, buf| {
                buf.push_str("\x1b[97m");
                buf.push(styled[revealed].ch);
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
        Scan {
            gradient: GRADIENT,
            bg_gradient: None,
        }
        .render_frame(&styled, 1, &mut buf);
        assert!(!buf.contains('a'));
    }

    #[test]
    fn chars_snap_after_cooldown() {
        let styled = parse_styled("a");
        let mut buf = String::new();
        let snap_frame = 3 + COOLDOWN_FRAMES;
        Scan {
            gradient: GRADIENT,
            bg_gradient: None,
        }
        .render_frame(&styled, snap_frame, &mut buf);
        assert!(buf.contains('a'));
    }
}
