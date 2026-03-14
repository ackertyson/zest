use std::fmt::Write;

use crate::style::{color256, StyledChar};

use super::Animation;

const COOLDOWN_FRAMES: usize = 14;

// Braille and block chars for a dot-matrix / fire-texture feel
const FLAME_CHARS: &[char] = &[
    '⣀', '⠠', '⠰', '⠸', '⠼', '⣤', '⣶', '⣿',
];

// 256-color gradient: bright orange-yellow → orange → red-orange → red → dark red
//   226=#ffff00  220=#ffd700  214=#ffaf00  208=#ff8700
//   202=#ff5f00  196=#ff0000  160=#d70000   88=#870000
pub(super) const GRADIENT: &[u8] = &[226, 220, 214, 208, 202, 196, 160, 88];

// white-blue → cyan → sky-blue → blue → dark navy
//   231=#ffffff  195=#d7ffff  159=#afffff  123=#87ffff   87=#5fffff   51=#00ffff
//    45=#00d7ff   39=#00afff   33=#0087ff   27=#005fff   21=#0000ff   18=#000087   17=#00005f
pub(super) const GRADIENT_BLUE: &[u8] = &[231, 195, 159, 123, 87, 51, 45, 39, 33, 27, 21, 18, 17];

// bright green → green → dark green
//   157=#afffaf  120=#87ff87   83=#5fff5f   46=#00ff00
//    40=#00d700   34=#00af00   28=#008700   22=#005f00
pub(super) const GRADIENT_GREEN: &[u8] = &[157, 120, 83, 46, 40, 34, 28, 22];

// pink/magenta → purple → dark purple
//   219=#ffafff  213=#ff87ff  207=#ff5fff  201=#ff00ff
//   165=#d700ff  129=#af00ff   93=#8700ff   57=#5f00ff   55=#5f00af
pub(super) const GRADIENT_PURPLE: &[u8] = &[219, 213, 207, 201, 165, 129, 93, 57, 55];

// solid hot pink — single color, no gradient: 198=#ff0087
pub(super) const GRADIENT_PINK: &[u8] = &[198];

pub struct Flames {
    pub(super) gradient: &'static [u8],
}

pub fn gradient_for(color: Option<&str>) -> Option<&'static [u8]> {
    match color {
        None | Some("orange") => Some(GRADIENT),
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

impl Animation for Flames {
    fn cooldown_frames(&self) -> usize { COOLDOWN_FRAMES }

    fn render_frame(&self, styled: &[StyledChar], frame: usize, buf: &mut String) {
        let n = styled.len();
        let revealed = super::revealed(frame, n);
        let last_content = super::last_content(styled);
        let has_leading = super::has_leading(frame, revealed, n, last_content);

        // Revealed chars: show flickering dot-matrix during cooldown, then snap to real color.
        // Characters at or past last_content (chevron, trailing whitespace) skip the flame
        // effect entirely and are always shown in their real color.
        for (i, sc) in styled[..revealed].iter().enumerate() {
            let age = frame.saturating_sub(i + 3);
            if age >= COOLDOWN_FRAMES || i >= last_content {
                buf.push_str("\x1b[0m");
                buf.push_str(&sc.color_prefix);
                buf.push(sc.ch);
            } else {
                color256(buf, super::cooldown_color(age, COOLDOWN_FRAMES, self.gradient));
                buf.push(flame_char(i, frame));
            }
        }

        // Leading edge: hottest color, flickering dot-matrix char
        if has_leading {
            color256(buf, self.gradient[0]);
            write!(buf, "{}", flame_char(revealed, frame)).unwrap();
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
        Flames { gradient: GRADIENT }.render_frame(&styled, 1, &mut buf);
        assert!(!buf.contains('a'));
    }

    #[test]
    fn leading_edge_present_at_frame_2() {
        let styled = parse_styled("ab");
        let mut buf = String::new();
        Flames { gradient: GRADIENT }.render_frame(&styled, 2, &mut buf);
        // Frame 2: 0 revealed, 1 leading edge char — buf should be non-empty
        assert!(buf.len() > "\x1b[0m".len());
    }

    #[test]
    fn chars_snap_after_cooldown() {
        let styled = parse_styled("a");
        let mut buf = String::new();
        // age for char 0 = frame - 3; fully cooled when age >= COOLDOWN_FRAMES
        let snap_frame = 3 + COOLDOWN_FRAMES;
        Flames { gradient: GRADIENT }.render_frame(&styled, snap_frame, &mut buf);
        assert!(buf.contains('a'));
    }
}
