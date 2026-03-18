use std::cell::OnceCell;
use std::fmt::Write;

use crate::style::{StyledChar, color256};

use super::Animation;

// Matrix-specific gradients: skewed darker than the shared ones so the bright
// flash is brief and characters spend most of their cooldown in deeper hues.
const MATRIX_GREEN: &[u8] = &[83, 46, 40, 34, 34, 28, 28, 22, 16];
const MATRIX_BLUE: &[u8] = &[87, 51, 45, 39, 33, 27, 21, 18, 17];
const MATRIX_ORANGE: &[u8] = &[208, 202, 196, 160, 160, 124, 88, 88, 52];
const MATRIX_PURPLE: &[u8] = &[207, 201, 165, 129, 93, 93, 57, 55, 55];
const MATRIX_RED: &[u8] = &[196, 160, 124, 124, 88, 88, 52, 52, 16];
const MATRIX_PINK: &[u8] = &[198];

const COOLDOWN_FRAMES: usize = 12;

const MATRIX_CHARS: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*+=<>?/";

pub struct Matrix {
    pub(super) gradient: &'static [u8],
    pub(super) bg_gradient: Option<&'static [u8]>,
    pub(super) glyph_frames: usize,
    pub(super) trigger: OnceCell<Vec<usize>>,
}

pub fn gradient_for(color: Option<&str>) -> Option<&'static [u8]> {
    match color {
        None | Some("green") => Some(MATRIX_GREEN),
        Some("blue") => Some(MATRIX_BLUE),
        Some("red") => Some(MATRIX_RED),
        Some("orange") => Some(MATRIX_ORANGE),
        Some("purple") => Some(MATRIX_PURPLE),
        Some("pink") => Some(MATRIX_PINK),
        _ => None,
    }
}

fn matrix_char(pos: usize, frame: usize) -> char {
    MATRIX_CHARS[super::hash(pos, frame) % MATRIX_CHARS.len()] as char
}

/// Fisher-Yates shuffle → inverted to get trigger\[pos\] = step at which that position starts cooling.
fn build_trigger(n: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..n).collect();
    for i in (1..n).rev() {
        let j = super::hash(i, 0x4d41_5458) % (i + 1);
        order.swap(i, j);
    }
    let mut trigger = vec![0usize; n];
    for (step, &pos) in order.iter().enumerate() {
        trigger[pos] = step;
    }
    trigger
}

impl Animation for Matrix {
    fn cooldown_frames(&self) -> usize {
        COOLDOWN_FRAMES
    }

    fn render_frame(&self, styled: &[StyledChar], frame: usize, buf: &mut String) {
        if frame < 2 {
            buf.push_str("\x1b[0m");
            return;
        }

        let n = styled.len();
        let gradient = self.gradient;
        let glyph_frames = self.glyph_frames;
        let trigger = self.trigger.get_or_init(|| build_trigger(n));
        let revealed = super::revealed(frame, n);
        let lc = super::last_content(styled);

        for (i, sc) in styled.iter().enumerate() {
            // Trailing whitespace snaps to real color immediately
            if i >= lc {
                buf.push_str("\x1b[0m");
                buf.push_str(&sc.color_prefix);
                buf.push(sc.ch);
                continue;
            }

            if trigger[i] < revealed {
                // Triggered — cooling or fully cooled
                let age = frame.saturating_sub(trigger[i] + 3);
                if age >= COOLDOWN_FRAMES {
                    buf.push_str("\x1b[0m");
                    buf.push_str(&sc.color_prefix);
                    buf.push(sc.ch);
                } else {
                    buf.push_str("\x1b[1m");
                    color256(buf, super::cooldown_color(age, COOLDOWN_FRAMES, gradient));
                    if let Some(bg) = self.bg_gradient {
                        if age < bg.len() {
                            write!(buf, "\x1b[48;5;{}m", bg[age]).unwrap();
                        } else {
                            buf.push_str("\x1b[49m");
                        }
                    }
                    buf.push(matrix_char(i, frame / glyph_frames));
                }
            } else {
                // Not yet triggered — scrambled glyph in hottest color
                buf.push_str("\x1b[1m");
                color256(buf, gradient[0]);
                buf.push(matrix_char(i, frame / glyph_frames));
            }
        }

        buf.push_str("\x1b[0m");
    }
}

#[cfg(test)]
mod tests {
    use crate::style::parse_styled;

    use super::*;

    fn test_matrix() -> Matrix {
        Matrix {
            gradient: MATRIX_GREEN,
            bg_gradient: None,
            glyph_frames: 6,
            trigger: OnceCell::new(),
        }
    }

    #[test]
    fn no_output_before_animation_starts() {
        let styled = parse_styled("abc");
        let mut buf = String::new();
        test_matrix().render_frame(&styled, 1, &mut buf);
        assert!(!buf.contains('a'));
    }

    #[test]
    fn chars_snap_after_cooldown() {
        let styled = parse_styled("a");
        let mut buf = String::new();
        let snap_frame = 3 + COOLDOWN_FRAMES;
        test_matrix().render_frame(&styled, snap_frame, &mut buf);
        assert!(buf.contains('a'));
    }
}
