use std::fmt::Write;

use crate::style::{color256, StyledChar};

use super::Animation;
use super::{GRADIENT_ORANGE, GRADIENT_BLUE, GRADIENT_GREEN, GRADIENT_PURPLE, GRADIENT_PINK};

const SPINNERS: [char; 4] = ['-', '\\', '|', '/'];
const COOLDOWN_FRAMES: usize = 12;

pub struct Sprout {
    pub(super) gradient: &'static [u8],
}

pub fn gradient_for(color: Option<&str>) -> Option<&'static [u8]> {
    match color {
        None | Some("green") => Some(GRADIENT_GREEN),
        Some("orange")       => Some(GRADIENT_ORANGE),
        Some("blue")         => Some(GRADIENT_BLUE),
        Some("purple")       => Some(GRADIENT_PURPLE),
        Some("pink")         => Some(GRADIENT_PINK),
        _                    => None,
    }
}

impl Animation for Sprout {
    fn cooldown_frames(&self) -> usize { COOLDOWN_FRAMES }

    fn render_frame(&self, styled: &[StyledChar], frame: usize, buf: &mut String) {
        let n = styled.len();
        let revealed = super::revealed(frame, n);
        // Don't draw the spinner on trailing whitespace or the final non-space character
        // (e.g. the chevron ❯). Find the index of the last non-whitespace char and stop before it.
        let last_content = super::last_content(styled);
        let has_spinner = super::has_leading(frame, revealed, n, last_content);

        // Revealed text: each char cools from HOT→target based on frames since it was revealed.
        for (i, sc) in styled[..revealed].iter().enumerate() {
            let age = frame.saturating_sub(i + 3);
            if age >= COOLDOWN_FRAMES {
                // Fully cooled: snap to the character's real color
                buf.push_str("\x1b[0m");
                buf.push_str(&sc.color_prefix);
            } else {
                color256(buf, super::cooldown_color(age, COOLDOWN_FRAMES, self.gradient));
            }
            buf.push(sc.ch);
        }

        // Spinner: bright white
        if has_spinner {
            write!(buf, "\x1b[97m{}", SPINNERS[(frame - 2) % SPINNERS.len()]).unwrap();
        }

        buf.push_str("\x1b[0m");
    }
}

#[cfg(test)]
mod tests {
    use crate::style::parse_styled;

    use super::*;

    #[test]
    fn render_frame_first_frame_empty() {
        let styled = parse_styled("abc");
        let mut buf = String::new();
        Sprout { gradient: GRADIENT_GREEN }.render_frame(&styled, 1, &mut buf);
        // Frame 1: nothing revealed, spinner not yet started
        assert!(!buf.contains('a'));
    }

    #[test]
    fn render_frame_reveals_chars() {
        let styled = parse_styled("ab");
        let mut buf = String::new();
        Sprout { gradient: GRADIENT_GREEN }.render_frame(&styled, 3, &mut buf);
        // Frame 3: 1 char revealed
        assert!(buf.contains('a'));
    }
}
