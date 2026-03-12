use crate::style::{color256, StyledChar};

use super::Animation;

const FLICKER_FRAMES: usize = 4;
const COOLDOWN_FRAMES: usize = 14; // 4 flicker + 10 steady glow

// 256-color gradient: white → magenta → purple
//   231=#ffffff  201=#ff00ff  207=#ff5fff  165=#d700ff  129=#af00ff   93=#8700ff   57=#5f00ff
const GRADIENT: &[u8] = &[231, 201, 207, 165, 129, 93, 57];

pub struct Neon;

impl Animation for Neon {
    fn cooldown_frames(&self) -> usize { COOLDOWN_FRAMES }

    fn render_frame(&self, styled: &[StyledChar], frame: usize, buf: &mut String) {
        let n = styled.len();
        let revealed = super::revealed(frame, n);
        let last_content = super::last_content(styled);
        let has_leading = super::has_leading(frame, revealed, n, last_content);

        for (i, sc) in styled[..revealed].iter().enumerate() {
            let age = frame.saturating_sub(i + 3);
            if age >= COOLDOWN_FRAMES {
                buf.push_str("\x1b[0m");
                buf.push_str(&sc.color_prefix);
                buf.push(sc.ch);
            } else if age < FLICKER_FRAMES {
                // Flicker phase: probability of being "on" increases with age
                let on = super::hash(i, frame) % 4 < age + 1;
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
