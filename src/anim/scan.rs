use crate::style::{color256, StyledChar};

use super::Animation;

const COOLDOWN_FRAMES: usize = 4;

// 256-color gradient: white → fading white
//   231=#ffffff  195=#d7ffff  189=#d7d7ff  183=#d7afff
const GRADIENT: &[u8] = &[231, 195, 189, 183];

pub struct Scan;

impl Animation for Scan {
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
            } else {
                color256(buf, super::cooldown_color(age, COOLDOWN_FRAMES, GRADIENT));
            }
            buf.push(sc.ch);
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
        Scan.render_frame(&styled, 1, &mut buf);
        assert!(!buf.contains('a'));
    }

    #[test]
    fn chars_snap_after_cooldown() {
        let styled = parse_styled("a");
        let mut buf = String::new();
        let snap_frame = 3 + COOLDOWN_FRAMES;
        Scan.render_frame(&styled, snap_frame, &mut buf);
        assert!(buf.contains('a'));
    }
}
