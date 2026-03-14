use crate::style::{color256, StyledChar};

use super::Animation;

const COOLDOWN_FRAMES: usize = 14;

const FLAME_CHARS: &[char] = &[
    '⣀', '⠠', '⠰', '⠸', '⠼', '⣤', '⣶', '⣿',
];

use super::{GRADIENT_ORANGE, GRADIENT_BLUE, GRADIENT_GREEN, GRADIENT_PURPLE, GRADIENT_PINK};

pub struct Flames {
    pub(super) gradient: &'static [u8],
}

pub fn gradient_for(color: Option<&str>) -> Option<&'static [u8]> {
    match color {
        None | Some("orange") => Some(GRADIENT_ORANGE),
        Some("blue")          => Some(GRADIENT_BLUE),
        Some("green")         => Some(GRADIENT_GREEN),
        Some("purple")        => Some(GRADIENT_PURPLE),
        Some("pink")          => Some(GRADIENT_PINK),
        _                     => None,
    }
}

fn flame_char(pos: usize, frame: usize) -> char {
    FLAME_CHARS[super::hash(pos, frame) % FLAME_CHARS.len()]
}

/// Sinusoidal wave heat — gives organic flowing color variation across the cooldown wake.
/// Coefficient 0.22 tuned for zest's 10ms frame rate.
fn wave_heat(pos: usize, frame: usize) -> f32 {
    let t = frame as f32 * 0.22;
    let p = pos as f32 * 0.7;
    let v = (t + p).sin() * 0.38 + (t * 1.9 - p * 0.5).sin() * 0.22 + 0.5;
    v.clamp(0.0, 1.0)
}

fn wave_color(pos: usize, frame: usize, gradient: &[u8]) -> u8 {
    let heat = wave_heat(pos, frame);
    let idx = ((1.0 - heat) * (gradient.len() - 1) as f32).round() as usize;
    gradient[idx.min(gradient.len() - 1)]
}

impl Animation for Flames {
    fn cooldown_frames(&self) -> usize { COOLDOWN_FRAMES }

    fn render_frame(&self, styled: &[StyledChar], frame: usize, buf: &mut String) {
        let gradient = self.gradient;
        let n = styled.len();
        let rev = super::revealed(frame, n);
        let lc = super::last_content(styled);
        let has_lead = super::has_leading(frame, rev, n, lc);

        for (i, sc) in styled[..rev].iter().enumerate() {
            let age = frame.saturating_sub(i + 3);
            if age >= COOLDOWN_FRAMES || i >= lc {
                buf.push_str("\x1b[0m");
                buf.push_str(&sc.color_prefix);
                buf.push(sc.ch);
            } else {
                color256(buf, wave_color(i, frame, gradient));
                buf.push(flame_char(i, frame));
            }
        }

        if has_lead {
            color256(buf, gradient[0]);
            buf.push(flame_char(rev, frame));
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
        Flames { gradient: GRADIENT_ORANGE }.render_frame(&styled, 1, &mut buf);
        assert!(!buf.contains('a'));
    }

    #[test]
    fn leading_edge_present_at_frame_2() {
        let styled = parse_styled("ab");
        let mut buf = String::new();
        Flames { gradient: GRADIENT_ORANGE }.render_frame(&styled, 2, &mut buf);
        assert!(buf.len() > "\x1b[0m".len());
    }

    #[test]
    fn chars_snap_after_cooldown() {
        let styled = parse_styled("a");
        let mut buf = String::new();
        let snap_frame = 3 + COOLDOWN_FRAMES;
        Flames { gradient: GRADIENT_ORANGE }.render_frame(&styled, snap_frame, &mut buf);
        assert!(buf.contains('a'));
    }
}
