/// Property tests for render_frame() across all animations.
///
/// These test the Animation trait directly — no PTY, no process spawning.
/// Two properties are checked:
///
/// 1. No panics: arbitrary input strings and arbitrary frame numbers never panic.
/// 2. Final frame completeness: at total_frames, the rendered output contains
///    every character from the input in order.
use proptest::prelude::*;
use zest::anim;
use zest::style::parse_styled;

const ANIMATIONS: &[&str] = &["flames", "matrix", "sprout", "scan", "shine"];

/// Strip ANSI escape sequences and return the remaining visible characters.
/// Independent of parse_styled — used as the oracle for final-frame checks.
fn visible_chars(s: &str) -> Vec<char> {
    let mut out = Vec::new();
    let mut in_esc = false;
    for ch in s.chars() {
        if ch == '\x1b' {
            in_esc = true;
        } else if in_esc {
            if ch.is_ascii_alphabetic() {
                in_esc = false;
            }
        } else {
            out.push(ch);
        }
    }
    out
}

proptest! {
    /// Arbitrary input (including malformed ANSI, control chars, high Unicode)
    /// and arbitrary frame numbers must never panic on any animation.
    #[test]
    fn render_frame_no_panic(
        anim_idx in 0usize..5,
        s in any::<String>(),
        frame in 0usize..200,
    ) {
        let anim = anim::resolve(ANIMATIONS[anim_idx], None, None, None, 4, 0).unwrap();
        let styled = parse_styled(&s);
        let mut buf = String::new();
        anim.render_frame(&styled, frame, &mut buf);
    }

    /// At total_frames, every input character must appear in the output in order —
    /// the full prompt is visible and complete. Checked across all animations.
    #[test]
    fn final_frame_contains_all_chars(
        anim_idx in 0usize..5,
        s in "[^\x00-\x1f\u{7f}-\u{9f}]{1,60}",
    ) {
        let anim = anim::resolve(ANIMATIONS[anim_idx], None, None, None, 4, 0).unwrap();
        let styled = parse_styled(&s);
        let total = anim.total_frames(&styled);
        let mut buf = String::new();
        anim.render_frame(&styled, total, &mut buf);
        let expected: Vec<char> = styled.iter().map(|sc| sc.ch).collect();
        prop_assert_eq!(visible_chars(&buf), expected);
    }
}
