use std::fmt::Write;

use crate::style::{color256, StyledChar};

use super::Animation;

const BAND_HALF: isize = 4;

// Foreground gradient from band center outward: white core → bright yellow → golden → dark gold
const FLASH_FG: &[u8] = &[231, 226, 220, 214, 178];
// Background gradient from band center outward: dark yellow → dark grey
const FLASH_BG: &[u8] = &[100, 58, 238, 237, 236];

pub struct Lightning;

impl Animation for Lightning {
    fn cooldown_frames(&self) -> usize { 0 }

    fn total_frames(&self, styled: &[StyledChar]) -> usize {
        // Band moves at half speed (1 char per 2 frames), so double the frame count.
        2 * (styled.len() + BAND_HALF as usize) + 2
    }

    fn render_frame(&self, styled: &[StyledChar], frame: usize, buf: &mut String) {
        // Half-speed: band center advances one char every two frames.
        let band_center = (frame as isize - 1) / 2;

        for (i, sc) in styled.iter().enumerate() {
            let dist = (i as isize - band_center).abs();
            if dist <= BAND_HALF {
                let d = dist as usize;
                color256(buf, FLASH_FG[d]);
                write!(buf, "\x1b[48;5;{}m", FLASH_BG[d]).unwrap();
            } else {
                buf.push_str("\x1b[0m");
                buf.push_str(&sc.color_prefix);
            }
            buf.push(sc.ch);
        }

        buf.push_str("\x1b[0m");
    }
}

#[cfg(test)]
mod tests {
    use crate::style::parse_styled;

    use super::*;

    #[test]
    fn all_chars_visible_every_frame() {
        let styled = parse_styled("hello");
        let mut buf = String::new();
        // Every frame should contain all characters
        for frame in 1..=Lightning.total_frames(&styled) {
            buf.clear();
            Lightning.render_frame(&styled, frame, &mut buf);
            for ch in "hello".chars() {
                assert!(buf.contains(ch), "frame {frame} missing '{ch}'");
            }
        }
    }

    #[test]
    fn band_passes_completely() {
        let styled = parse_styled("hello");
        let last_frame = Lightning.total_frames(&styled);
        let mut buf = String::new();
        Lightning.render_frame(&styled, last_frame, &mut buf);
        // Last frame: band is past all chars, so no flash colors should appear
        for color in FLASH_FG {
            assert!(!buf.contains(&format!("38;5;{color}")),
                "flash color {color} still present in last frame");
        }
    }
}
