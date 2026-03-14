use crate::style::{color256, StyledChar};

use super::Animation;

const COOLDOWN_FRAMES: usize = 12;

const MATRIX_CHARS: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*+=<>?/";

// 256-color gradient: bright green → dark green
//   118=#87ff00   82=#5fff00   46=#00ff00   40=#00d700   34=#00af00   28=#008700
pub(super) const GRADIENT: &[u8] = &[118, 82, 46, 40, 34, 28];

// white-blue → cyan → sky-blue → blue → dark navy
pub(super) const GRADIENT_BLUE: &[u8] = &[231, 195, 159, 123, 87, 51, 45, 39, 33, 27, 21, 18, 17];

// bright red → dark red
pub(super) const GRADIENT_RED: &[u8] = &[196, 160, 124, 88, 52];

// bright orange-yellow → orange → red-orange → red → dark red
pub(super) const GRADIENT_ORANGE: &[u8] = &[226, 220, 214, 208, 202, 196, 160, 88];

// pink/magenta → purple → dark purple
pub(super) const GRADIENT_PURPLE: &[u8] = &[219, 213, 207, 201, 165, 129, 93, 57, 55];

// solid hot pink — single color: 198=#ff0087
pub(super) const GRADIENT_PINK: &[u8] = &[198];

pub struct Matrix {
    pub(super) gradient: &'static [u8],
}

pub fn gradient_for(color: Option<&str>) -> Option<&'static [u8]> {
    match color {
        None | Some("green") => Some(GRADIENT),
        Some("blue")         => Some(GRADIENT_BLUE),
        Some("red")          => Some(GRADIENT_RED),
        Some("orange")       => Some(GRADIENT_ORANGE),
        Some("purple")       => Some(GRADIENT_PURPLE),
        Some("pink")         => Some(GRADIENT_PINK),
        _                    => None,
    }
}

fn matrix_char(pos: usize, frame: usize) -> char {
    MATRIX_CHARS[super::hash(pos, frame) % MATRIX_CHARS.len()] as char
}

impl Animation for Matrix {
    fn cooldown_frames(&self) -> usize { COOLDOWN_FRAMES }

    fn render_frame(&self, styled: &[StyledChar], frame: usize, buf: &mut String) {
        let n = styled.len();
        let revealed = super::revealed(frame, n);
        let last_content = super::last_content(styled);
        let has_leading = super::has_leading(frame, revealed, n, last_content);

        for (i, sc) in styled[..revealed].iter().enumerate() {
            let age = frame.saturating_sub(i + 3);
            if age >= COOLDOWN_FRAMES || i >= last_content {
                buf.push_str("\x1b[0m");
                buf.push_str(&sc.color_prefix);
                buf.push(sc.ch);
            } else {
                color256(buf, super::cooldown_color(age, COOLDOWN_FRAMES, self.gradient));
                buf.push(matrix_char(i, frame));
            }
        }

        if has_leading {
            color256(buf, self.gradient[0]);
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
        Matrix { gradient: GRADIENT }.render_frame(&styled, 1, &mut buf);
        assert!(!buf.contains('a'));
    }

    #[test]
    fn chars_snap_after_cooldown() {
        let styled = parse_styled("a");
        let mut buf = String::new();
        let snap_frame = 3 + COOLDOWN_FRAMES;
        Matrix { gradient: GRADIENT }.render_frame(&styled, snap_frame, &mut buf);
        assert!(buf.contains('a'));
    }
}
