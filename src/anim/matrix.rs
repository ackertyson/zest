use crate::style::{color256, StyledChar};

use super::Animation;

const FRAME_DELAY_MS: u64 = 10;
const COOLDOWN_FRAMES: usize = 12;

const MATRIX_CHARS: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*+=<>?/";

// 256-color gradient: bright green → dark green
//   118=#87ff00   82=#5fff00   46=#00ff00   40=#00d700   34=#00af00   28=#008700
const GRADIENT: &[u8] = &[118, 82, 46, 40, 34, 28];

pub struct Matrix;

fn matrix_char(pos: usize, frame: usize) -> char {
    let mut h = pos.wrapping_add(frame.wrapping_mul(0x9e3779b97f4a7c15));
    h = (h ^ (h >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94d049bb133111eb);
    h ^= h >> 31;
    MATRIX_CHARS[h % MATRIX_CHARS.len()] as char
}

impl Animation for Matrix {
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
            if age >= COOLDOWN_FRAMES || i >= last_content {
                buf.push_str("\x1b[0m");
                buf.push_str(&sc.color_prefix);
                buf.push(sc.ch);
            } else {
                color256(buf, super::cooldown_color(age, COOLDOWN_FRAMES, GRADIENT));
                buf.push(matrix_char(i, frame));
            }
        }

        if has_leading {
            color256(buf, GRADIENT[0]);
            buf.push(matrix_char(revealed, frame));
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
        Matrix.render_frame(&styled, 1, &mut buf);
        assert!(!buf.contains('a'));
    }

    #[test]
    fn chars_snap_after_cooldown() {
        let styled = parse_styled("a");
        let mut buf = String::new();
        let snap_frame = 3 + COOLDOWN_FRAMES;
        Matrix.render_frame(&styled, snap_frame, &mut buf);
        assert!(buf.contains('a'));
    }
}
