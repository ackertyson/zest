use std::fmt::Write;

use crate::style::{StyledChar, color256};

use super::Animation;

const BAND_HALF: isize = 4;

// Default: yellow/gold flash band (white core → bright yellow → gold → dark gold)
const FLASH_FG: &[u8] = &[231, 226, 220, 214, 178];
const FLASH_BG: &[u8] = &[100, 58, 238, 237, 236];

// Blue: white core → pale cyan → bright cyan → blue → dark blue
const FLASH_FG_BLUE: &[u8] = &[231, 195, 51, 39, 27];
const FLASH_BG_BLUE: &[u8] = &[18, 17, 238, 237, 236];

// Green: white core → pale green → bright green → medium → dark green
const FLASH_FG_GREEN: &[u8] = &[231, 157, 46, 34, 28];
const FLASH_BG_GREEN: &[u8] = &[22, 238, 237, 236, 235];

// Orange: white core → peach → orange → dark orange → red-orange
const FLASH_FG_ORANGE: &[u8] = &[231, 222, 214, 208, 202];
const FLASH_BG_ORANGE: &[u8] = &[130, 94, 238, 237, 236];

// Purple: white core → light pink → magenta → purple → dark purple
const FLASH_FG_PURPLE: &[u8] = &[231, 219, 201, 129, 93];
const FLASH_BG_PURPLE: &[u8] = &[54, 238, 237, 236, 235];

// Pink: white core → very light pink → pink → hot pink → deep pink
const FLASH_FG_PINK: &[u8] = &[231, 225, 219, 213, 207];
const FLASH_BG_PINK: &[u8] = &[89, 238, 237, 236, 235];

// Red: white core → light salmon → coral → red → dark red
const FLASH_FG_RED: &[u8] = &[231, 224, 210, 196, 160];
const FLASH_BG_RED: &[u8] = &[88, 238, 237, 236, 235];

pub struct Shine {
    pub(super) flash_fg: &'static [u8],
    pub(super) flash_bg: Option<&'static [u8]>,
}

pub fn gradient_for(color: Option<&str>) -> Option<(&'static [u8], &'static [u8])> {
    match color {
        None | Some("yellow") => Some((FLASH_FG, FLASH_BG)),
        Some("blue") => Some((FLASH_FG_BLUE, FLASH_BG_BLUE)),
        Some("green") => Some((FLASH_FG_GREEN, FLASH_BG_GREEN)),
        Some("orange") => Some((FLASH_FG_ORANGE, FLASH_BG_ORANGE)),
        Some("purple") => Some((FLASH_FG_PURPLE, FLASH_BG_PURPLE)),
        Some("pink") => Some((FLASH_FG_PINK, FLASH_BG_PINK)),
        Some("red") => Some((FLASH_FG_RED, FLASH_BG_RED)),
        _ => None,
    }
}

impl Animation for Shine {
    fn cooldown_frames(&self) -> usize {
        0
    }

    fn total_frames(&self, styled: &[StyledChar]) -> usize {
        styled.len() + BAND_HALF as usize + 1
    }

    fn render_frame(&self, styled: &[StyledChar], frame: usize, buf: &mut String) {
        let band_center = frame as isize - 1;

        for (i, sc) in styled.iter().enumerate() {
            let dist = (i as isize - band_center).unsigned_abs();
            let in_fg = dist < self.flash_fg.len();
            let in_bg = self.flash_bg.is_some_and(|bg| dist < bg.len());
            if in_fg || in_bg {
                if in_fg {
                    color256(buf, self.flash_fg[dist]);
                }
                if in_bg {
                    write!(buf, "\x1b[48;5;{}m", self.flash_bg.unwrap()[dist]).unwrap();
                }
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

    fn default_shine() -> Shine {
        Shine {
            flash_fg: FLASH_FG,
            flash_bg: Some(FLASH_BG),
        }
    }

    #[test]
    fn all_chars_visible_every_frame() {
        let styled = parse_styled("hello");
        let shine = default_shine();
        let mut buf = String::new();
        // Every frame should contain all characters
        for frame in 1..=shine.total_frames(&styled) {
            buf.clear();
            shine.render_frame(&styled, frame, &mut buf);
            for ch in "hello".chars() {
                assert!(buf.contains(ch), "frame {frame} missing '{ch}'");
            }
        }
    }

    #[test]
    fn band_passes_completely() {
        let styled = parse_styled("hello");
        let shine = default_shine();
        let last_frame = shine.total_frames(&styled);
        let mut buf = String::new();
        shine.render_frame(&styled, last_frame, &mut buf);
        // Last frame: band is past all chars, so no flash colors should appear
        for color in FLASH_FG {
            assert!(
                !buf.contains(&format!("38;5;{color}")),
                "flash color {color} still present in last frame"
            );
        }
    }
}
