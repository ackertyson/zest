use std::fmt::Write;

use crate::style::{color256, StyledChar};

use super::Animation;

const SPINNERS: [char; 4] = ['-', '\\', '|', '/'];
const FRAME_DELAY_MS: u64 = 10;
const COOLDOWN_FRAMES: usize = 12;

// 256-color gradient from hot (bright greenish-white) to resting dark green.
// Each entry is an ANSI 256-color index; index 0 is freshly revealed, last is fully cooled.
//   194 = #d7ffd7   157 = #afffaf   120 = #87ff87
//    83 = #5fff5f    46 = #00ff00    40 = #00d700    34 = #00af00
const GRADIENT: &[u8] = &[194, 157, 120, 83, 46, 40, 34];

pub struct GreenFlash;

fn cooldown_color(age: usize) -> u8 {
    let steps = GRADIENT.len() - 1;
    let idx = ((age * steps) / COOLDOWN_FRAMES).min(steps);
    GRADIENT[idx]
}

impl Animation for GreenFlash {
    fn total_frames(&self, n: usize) -> usize {
        n + COOLDOWN_FRAMES
    }

    fn frame_delay_ms(&self) -> u64 {
        FRAME_DELAY_MS
    }

    fn render_frame(&self, styled: &[StyledChar], n: usize, frame: usize, buf: &mut String) {
        let revealed = if frame >= 2 { (frame - 2).min(n) } else { 0 };
        let space_for_spinner = n.saturating_sub(revealed);
        let has_spinner = frame >= 2 && space_for_spinner > 0;

        // Revealed text: each char cools from HOT→target based on frames since it was revealed.
        for (i, sc) in styled[..revealed].iter().enumerate() {
            let age = frame.saturating_sub(i + 3);
            if age >= COOLDOWN_FRAMES {
                // Fully cooled: snap to the character's real color
                buf.push_str("\x1b[0m");
                buf.push_str(&sc.color_prefix);
            } else {
                color256(buf, cooldown_color(age));
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
        GreenFlash.render_frame(&styled, 3, 1, &mut buf);
        // Frame 1: nothing revealed, spinner not yet started
        assert!(!buf.contains('a'));
    }

    #[test]
    fn render_frame_reveals_chars() {
        let styled = parse_styled("ab");
        let mut buf = String::new();
        GreenFlash.render_frame(&styled, 2, 3, &mut buf);
        // Frame 3: 1 char revealed
        assert!(buf.contains('a'));
    }
}
