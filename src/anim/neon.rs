use crate::style::{color256, StyledChar};

use super::Animation;

const FRAME_DELAY_MS: u64 = 10;
const FLICKER_FRAMES: usize = 4;
const COOLDOWN_FRAMES: usize = 14; // 4 flicker + 10 steady glow

// 256-color gradient: white → magenta → purple
//   231=#ffffff  201=#ff00ff  207=#ff5fff  165=#d700ff  129=#af00ff   93=#8700ff   57=#5f00ff
const GRADIENT: &[u8] = &[231, 201, 207, 165, 129, 93, 57];

pub struct Neon;

fn neon_hash(pos: usize, frame: usize) -> u64 {
    let mut h = pos.wrapping_add(frame.wrapping_mul(0x9e3779b97f4a7c15));
    h = (h ^ (h >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94d049bb133111eb);
    (h ^ (h >> 31)) as u64
}

impl Animation for Neon {
    fn total_frames(&self, styled: &[StyledChar]) -> usize {
        styled.len() + COOLDOWN_FRAMES
    }

    fn frame_delay_ms(&self) -> u64 {
        FRAME_DELAY_MS
    }

    fn render_frame(&self, styled: &[StyledChar], frame: usize, buf: &mut String) {
        let n = styled.len();
        let revealed = if frame >= 2 { (frame - 2).min(n) } else { 0 };
        let last_content = styled
            .iter()
            .rposition(|sc| !sc.ch.is_whitespace())
            .unwrap_or(n);
        let has_leading = frame >= 2 && revealed < n && revealed < last_content;

        for (i, sc) in styled[..revealed].iter().enumerate() {
            let age = frame.saturating_sub(i + 3);
            if age >= COOLDOWN_FRAMES {
                buf.push_str("\x1b[0m");
                buf.push_str(&sc.color_prefix);
                buf.push(sc.ch);
            } else if age < FLICKER_FRAMES {
                // Flicker phase: probability of being "on" increases with age
                let on = neon_hash(i, frame) % 4 < (age + 1) as u64;
                if on {
                    color256(buf, GRADIENT[0]);
                    buf.push(sc.ch);
                } else {
                    buf.push(' ');
                }
            } else {
                // Steady glow: cool through gradient
                let glow_age = age - FLICKER_FRAMES;
                let glow_frames = COOLDOWN_FRAMES - FLICKER_FRAMES;
                color256(buf, super::cooldown_color(glow_age, glow_frames, GRADIENT));
                buf.push(sc.ch);
            }
        }

        if has_leading {
            buf.push_str("\x1b[97m");
            buf.push(styled[revealed].ch);
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
        Neon.render_frame(&styled, 1, &mut buf);
        assert!(!buf.contains('a'));
    }

    #[test]
    fn chars_snap_after_cooldown() {
        let styled = parse_styled("a");
        let mut buf = String::new();
        let snap_frame = 3 + COOLDOWN_FRAMES;
        Neon.render_frame(&styled, snap_frame, &mut buf);
        assert!(buf.contains('a'));
    }
}
