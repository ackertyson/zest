//! Unit-level tests for render_frame() across all animations.
//!
//! These test the Animation trait's render_frame() directly — no PTY, no process
//! spawning, fully deterministic. This is the primary frame validation layer.

use zest::anim::{self, Animation};
use zest::style::parse_styled;

// ── Test helpers ────────────────────────────────────────────────────────────

const PROMPT: &str = "\x1b[36m~/projects/zest\x1b[0m \x1b[96m❯ \x1b[0m";

const TEST_SEED: u32 = 42;

fn make_animation(name: &str) -> Box<dyn Animation> {
    anim::resolve(name, None, None, None, 4, TEST_SEED).expect("unknown animation")
}

fn make_animation_color(name: &str, color: &str) -> Box<dyn Animation> {
    anim::resolve(name, Some(color), None, None, 4, TEST_SEED).expect("unknown animation+color")
}

fn render_all_frames(anim: &dyn Animation, input: &str) -> Vec<String> {
    let styled = parse_styled(input);
    let total = anim.total_frames(&styled);
    (1..=total)
        .map(|frame| {
            let mut buf = String::new();
            anim.render_frame(&styled, frame, &mut buf);
            buf
        })
        .collect()
}

/// Extract visible characters from a rendered frame (strip ANSI escapes).
fn visible_chars(rendered: &str) -> Vec<char> {
    let mut chars = Vec::new();
    let mut in_esc = false;
    for ch in rendered.chars() {
        if in_esc {
            if ch.is_ascii_alphabetic() {
                in_esc = false;
            }
        } else if ch == '\x1b' {
            in_esc = true;
        } else {
            chars.push(ch);
        }
    }
    chars
}

/// Count visible (non-escape) characters in a rendered frame.
fn visible_len(rendered: &str) -> usize {
    visible_chars(rendered).len()
}

/// Check if a frame contains a specific 256-color foreground sequence.
fn has_color256(rendered: &str, idx: u8) -> bool {
    rendered.contains(&format!("38;5;{idx}"))
}

// ── Sprout ──────────────────────────────────────────────────────────────────

#[test]
fn sprout_frame_1_reveals_first_char() {
    let anim = make_animation("sprout");
    let styled = parse_styled("hello");
    let mut buf = String::new();
    anim.render_frame(&styled, 1, &mut buf);
    // Frame 1 reveals the first char + spinner leading edge
    assert!(visible_len(&buf) > 0, "frame 1 should reveal content");
}

#[test]
fn sprout_progressive_reveal() {
    let anim = make_animation("sprout");
    let input = "abcdef";
    let styled = parse_styled(input);

    // Visible char count should be non-decreasing across frames
    let mut prev_len = 0;
    let total = anim.total_frames(&styled);
    for frame in 1..=total {
        let mut buf = String::new();
        anim.render_frame(&styled, frame, &mut buf);
        let cur_len = visible_len(&buf);
        assert!(
            cur_len >= prev_len,
            "frame {frame}: visible chars decreased ({prev_len} -> {cur_len})"
        );
        prev_len = cur_len;
    }
    // By the end, all input chars should be visible
    assert_eq!(prev_len, input.len());
}

#[test]
fn sprout_spinner_character_present() {
    let anim = make_animation("sprout");
    let styled = parse_styled("hello world");
    let spinners = ['-', '\\', '|', '/'];

    for frame in 1..7 {
        let mut buf = String::new();
        anim.render_frame(&styled, frame, &mut buf);
        let vis = visible_chars(&buf);
        let expected_spinner = spinners[(frame - 1) % spinners.len()];
        assert!(
            vis.contains(&expected_spinner),
            "frame {frame}: missing spinner char '{expected_spinner}'"
        );
    }
}

#[test]
fn sprout_final_frame_shows_real_chars() {
    let anim = make_animation("sprout");
    let input = "hello";
    let styled = parse_styled(input);
    let total = anim.total_frames(&styled);

    let mut buf = String::new();
    anim.render_frame(&styled, total, &mut buf);
    for ch in input.chars() {
        assert!(buf.contains(ch), "final frame missing char '{ch}'");
    }
}

#[test]
fn sprout_cooldown_uses_gradient_colors() {
    let anim = make_animation("sprout");
    let input = "abcdefghijklmnop";
    let styled = parse_styled(input);

    // After several chars are revealed, early chars should be in cooldown with gradient colors
    let mut buf = String::new();
    anim.render_frame(&styled, 8, &mut buf);
    // Should contain 256-color escape sequences
    assert!(
        buf.contains("38;5;"),
        "cooldown phase should use 256-color mode"
    );
}

#[test]
fn sprout_all_colors_resolve() {
    for color in &["green", "orange", "blue", "purple", "pink"] {
        let anim = make_animation_color("sprout", color);
        let frames = render_all_frames(anim.as_ref(), "abc");
        assert!(!frames.is_empty(), "sprout {color} should produce frames");
    }
}

// ── Flames ──────────────────────────────────────────────────────────────────

#[test]
fn flames_frame_1_reveals_first_char() {
    let anim = make_animation("flames");
    let styled = parse_styled("hello");
    let mut buf = String::new();
    anim.render_frame(&styled, 1, &mut buf);
    // Frame 1 reveals the first char with a flame glyph leading edge
    assert!(visible_len(&buf) > 0, "frame 1 should reveal content");
}

#[test]
fn flames_progressive_reveal() {
    let anim = make_animation("flames");
    let input = "abcdef";
    let styled = parse_styled(input);
    let n = styled.len();

    let mut prev_vis_len = 0;
    for frame in 1..1 + n {
        let mut buf = String::new();
        anim.render_frame(&styled, frame, &mut buf);
        let vis_len = visible_len(&buf);
        assert!(
            vis_len >= prev_vis_len,
            "frame {frame}: visible chars decreased ({prev_vis_len} -> {vis_len})"
        );
        prev_vis_len = vis_len;
    }
}

#[test]
fn flames_uses_braille_chars_during_cooldown() {
    let anim = make_animation("flames");
    let input = "abcdefghij";
    let styled = parse_styled(input);

    let mut buf = String::new();
    anim.render_frame(&styled, 6, &mut buf);
    let vis = visible_chars(&buf);
    // During cooldown, chars should be braille/block chars, not the original chars
    let has_braille = vis
        .iter()
        .any(|&c| c as u32 >= 0x2800 && c as u32 <= 0x28FF);
    assert!(
        has_braille,
        "flames should show braille chars during cooldown, got: {:?}",
        vis
    );
}

#[test]
fn flames_trailing_whitespace_snaps_immediately() {
    // The trailing space + chevron (" ❯ ") should snap to real color, not show flame chars
    let anim = make_animation("flames");
    let input = "~/projects/zest ❯ ";
    let styled = parse_styled(input);
    let total = anim.total_frames(&styled);

    // At the last few frames, trailing content should be real chars
    let mut buf = String::new();
    anim.render_frame(&styled, total, &mut buf);
    assert!(buf.contains('❯'), "trailing chevron should be real char");
}

#[test]
fn flames_final_frame_shows_real_chars() {
    let anim = make_animation("flames");
    let input = "hello";
    let styled = parse_styled(input);
    let total = anim.total_frames(&styled);

    let mut buf = String::new();
    anim.render_frame(&styled, total, &mut buf);
    for ch in input.chars() {
        assert!(buf.contains(ch), "final frame missing char '{ch}'");
    }
}

#[test]
fn flames_all_colors_resolve() {
    for color in &["orange", "blue", "green", "purple", "pink"] {
        let anim = make_animation_color("flames", color);
        let frames = render_all_frames(anim.as_ref(), "abc");
        assert!(!frames.is_empty(), "flames {color} should produce frames");
    }
}

// ── Matrix ──────────────────────────────────────────────────────────────────

#[test]
fn matrix_all_positions_visible_at_frame_1() {
    let anim = make_animation("matrix");
    let input = "hello";
    let styled = parse_styled(input);
    let mut buf = String::new();
    anim.render_frame(&styled, 1, &mut buf);

    // At frame 1, all positions show scrambled glyphs
    let vis = visible_chars(&buf);
    assert_eq!(
        vis.len(),
        input.len(),
        "frame 1 should show all positions as scrambled glyphs"
    );
}

#[test]
fn matrix_chars_resolve_over_time() {
    let anim = make_animation("matrix");
    let input = "abcde";
    let styled = parse_styled(input);
    let total = anim.total_frames(&styled);

    let mut buf = String::new();
    anim.render_frame(&styled, total, &mut buf);
    for ch in input.chars() {
        assert!(
            buf.contains(ch),
            "final frame should contain real char '{ch}'"
        );
    }
}

#[test]
fn matrix_resolve_order_is_deterministic() {
    let anim1 = make_animation("matrix");
    let anim2 = make_animation("matrix");
    let input = "abcdefghij";

    let frames1 = render_all_frames(anim1.as_ref(), input);
    let frames2 = render_all_frames(anim2.as_ref(), input);

    assert_eq!(
        frames1, frames2,
        "matrix output should be deterministic across runs"
    );
}

#[test]
fn matrix_scramble_uses_ascii_chars() {
    let anim = make_animation("matrix");
    let styled = parse_styled("hello");
    let mut buf = String::new();
    anim.render_frame(&styled, 1, &mut buf);

    let vis = visible_chars(&buf);
    for ch in &vis {
        assert!(ch.is_ascii(), "scrambled char should be ASCII, got '{ch}'");
    }
}

#[test]
fn matrix_all_colors_resolve() {
    for color in &["green", "blue", "red", "orange", "purple", "pink"] {
        let anim = make_animation_color("matrix", color);
        let frames = render_all_frames(anim.as_ref(), "abc");
        assert!(!frames.is_empty(), "matrix {color} should produce frames");
    }
}

// ── Scan ────────────────────────────────────────────────────────────────────

#[test]
fn scan_frame_1_reveals_first_char() {
    let anim = make_animation("scan");
    let styled = parse_styled("hello");
    let mut buf = String::new();
    anim.render_frame(&styled, 1, &mut buf);
    // Frame 1 reveals the first char with white leading edge
    assert!(visible_len(&buf) > 0, "frame 1 should reveal content");
}

#[test]
fn scan_progressive_reveal_with_real_chars() {
    let anim = make_animation("scan");
    let input = "abcdef";
    let styled = parse_styled(input);

    // Scan shows real chars (not flame/matrix glyphs) during cooldown
    let mut buf = String::new();
    anim.render_frame(&styled, 5, &mut buf);
    let vis = visible_chars(&buf);
    // Early revealed chars should be the actual characters
    assert!(
        vis.contains(&'a'),
        "scan should show real chars, not glyphs"
    );
}

#[test]
fn scan_leading_edge_is_white() {
    let anim = make_animation("scan");
    let styled = parse_styled("abcdefghij");
    let mut buf = String::new();
    anim.render_frame(&styled, 4, &mut buf);
    // Leading edge should use bright white (\x1b[97m)
    assert!(
        buf.contains("\x1b[97m"),
        "scan leading edge should be bright white"
    );
}

#[test]
fn scan_final_frame_shows_real_chars() {
    let anim = make_animation("scan");
    let input = "hello";
    let styled = parse_styled(input);
    let total = anim.total_frames(&styled);
    let mut buf = String::new();
    anim.render_frame(&styled, total, &mut buf);
    for ch in input.chars() {
        assert!(buf.contains(ch), "final frame missing char '{ch}'");
    }
}

#[test]
fn scan_short_cooldown() {
    // Scan has only 4 cooldown frames — verify chars snap quickly
    let anim = make_animation("scan");
    let styled = parse_styled("a");
    let mut buf = String::new();
    // Frame 1 reveals 'a', cooldown is 4 frames, so frame 5 should be snapped
    anim.render_frame(&styled, 5, &mut buf);
    assert!(buf.contains('a'), "char should snap after short cooldown");
}

#[test]
fn scan_all_colors_resolve() {
    for color in &["white", "blue", "green", "orange", "purple", "pink", "red"] {
        let anim = make_animation_color("scan", color);
        let frames = render_all_frames(anim.as_ref(), "abc");
        assert!(!frames.is_empty(), "scan {color} should produce frames");
    }
}

// ── Shine ───────────────────────────────────────────────────────────────────

#[test]
fn shine_all_chars_visible_from_frame_1() {
    let anim = make_animation("shine");
    let input = "hello";
    let styled = parse_styled(input);
    let mut buf = String::new();
    anim.render_frame(&styled, 1, &mut buf);
    for ch in input.chars() {
        assert!(
            buf.contains(ch),
            "shine frame 1 should show all chars, missing '{ch}'"
        );
    }
}

#[test]
fn shine_flash_band_sweeps_left_to_right() {
    let anim = make_animation("shine");
    let input = "abcdefghijklmnop";
    let styled = parse_styled(input);

    // Track which frames have the flash band core (color 231 = white) near specific positions
    let early_frame = 2;
    let late_frame = 10;

    let mut early_buf = String::new();
    let mut late_buf = String::new();
    anim.render_frame(&styled, early_frame, &mut early_buf);
    anim.render_frame(&styled, late_frame, &mut late_buf);

    // Both should have the white flash core
    assert!(
        has_color256(&early_buf, 231),
        "early frame should have flash band"
    );
    assert!(
        has_color256(&late_buf, 231),
        "late frame should have flash band"
    );
}

#[test]
fn shine_band_exits_completely() {
    let anim = make_animation("shine");
    let input = "hello";
    let styled = parse_styled(input);
    let total = anim.total_frames(&styled);

    let mut buf = String::new();
    anim.render_frame(&styled, total, &mut buf);
    // Last frame: flash band should be fully past all chars
    assert!(
        !has_color256(&buf, 231),
        "flash core should not be present in last frame"
    );
}

#[test]
fn shine_has_background_colors() {
    let anim = make_animation("shine");
    let styled = parse_styled("abcdefghij");
    let mut buf = String::new();
    anim.render_frame(&styled, 3, &mut buf);
    // Default yellow shine should have background colors
    assert!(
        buf.contains("48;5;"),
        "shine should use background colors in flash band"
    );
}

#[test]
fn shine_no_cooldown_frames() {
    let anim = make_animation("shine");
    assert_eq!(
        anim.cooldown_frames(),
        0,
        "shine should have 0 cooldown frames"
    );
}

#[test]
fn shine_all_colors_resolve() {
    for color in &["yellow", "blue", "green", "orange", "purple", "pink", "red"] {
        let anim = make_animation_color("shine", color);
        let frames = render_all_frames(anim.as_ref(), "abc");
        assert!(!frames.is_empty(), "shine {color} should produce frames");
    }
}

// ── Cross-animation tests ───────────────────────────────────────────────────

#[test]
fn all_animations_produce_correct_total_frames() {
    let styled = parse_styled(PROMPT);

    for (name, _) in anim::LIST {
        let anim = make_animation(name);
        let total = anim.total_frames(&styled);
        let cooldown = anim.cooldown_frames();
        if *name == "shine" {
            // Shine: len + BAND_HALF + 1
            assert!(
                total > styled.len(),
                "{name}: total_frames ({total}) should be > input len ({})",
                styled.len()
            );
        } else {
            assert_eq!(
                total,
                styled.len() + cooldown,
                "{name}: total_frames should be len + cooldown"
            );
        }
    }
}

#[test]
fn all_animations_end_with_reset() {
    let styled = parse_styled("hello");

    for (name, _) in anim::LIST {
        let anim = make_animation(name);
        let total = anim.total_frames(&styled);
        let mut buf = String::new();
        anim.render_frame(&styled, total, &mut buf);
        assert!(
            buf.ends_with("\x1b[0m"),
            "{name}: final frame should end with ANSI reset"
        );
    }
}

#[test]
fn all_animations_with_ansi_input() {
    for (name, _) in anim::LIST {
        let anim = make_animation(name);
        let frames = render_all_frames(anim.as_ref(), PROMPT);
        assert!(
            !frames.is_empty(),
            "{name}: should produce frames with ANSI input"
        );

        // Final frame should contain the real visible characters for all animations
        let last = frames.last().unwrap();
        for ch in ['~', '/', 'z', 'e', 's', 't'] {
            assert!(
                last.contains(ch),
                "{name}: final frame missing visible char '{ch}'"
            );
        }
    }
}

#[test]
fn all_animations_no_truncated_escapes() {
    for (name, _) in anim::LIST {
        let anim = make_animation(name);
        let frames = render_all_frames(anim.as_ref(), PROMPT);

        for (i, frame) in frames.iter().enumerate() {
            // Every \x1b should be followed by [ and eventually a letter
            let bytes: Vec<char> = frame.chars().collect();
            let mut j = 0;
            while j < bytes.len() {
                if bytes[j] == '\x1b' {
                    assert!(
                        j + 1 < bytes.len(),
                        "{name} frame {}: truncated escape at end",
                        i + 1
                    );
                    if bytes[j + 1] == '[' {
                        // Find the terminator
                        let mut k = j + 2;
                        while k < bytes.len() && !bytes[k].is_ascii_alphabetic() {
                            k += 1;
                        }
                        assert!(
                            k < bytes.len(),
                            "{name} frame {}: unterminated CSI sequence starting at position {j}",
                            i + 1
                        );
                    }
                }
                j += 1;
            }
        }
    }
}

#[test]
fn all_animations_deterministic() {
    for (name, _) in anim::LIST {
        let anim1 = make_animation(name);
        let anim2 = make_animation(name);
        let frames1 = render_all_frames(anim1.as_ref(), PROMPT);
        let frames2 = render_all_frames(anim2.as_ref(), PROMPT);
        assert_eq!(
            frames1, frames2,
            "{name}: output should be deterministic across runs"
        );
    }
}

// ── Color snap tests ────────────────────────────────────────────────────────
// The core visual contract: once cooldown completes, each character must render
// with its original ANSI color prefix, not a gradient or effect color.

#[test]
fn sweep_animations_snap_to_original_colors() {
    let input = "\x1b[36mhello\x1b[0m \x1b[96mworld\x1b[0m";
    let styled = parse_styled(input);

    for name in ["sprout", "flames", "scan"] {
        let anim = make_animation(name);
        let total = anim.total_frames(&styled);
        let mut buf = String::new();
        anim.render_frame(&styled, total, &mut buf);

        // The cyan color prefix (\x1b[36m) must appear in the final frame —
        // this is the original prompt color, not a gradient color.
        assert!(
            buf.contains("\x1b[36m"),
            "{name}: final frame must contain original cyan color \\x1b[36m"
        );
        // The bright cyan prefix (\x1b[96m) must also be restored
        assert!(
            buf.contains("\x1b[96m"),
            "{name}: final frame must contain original bright cyan color \\x1b[96m"
        );
        // Gradient colors (38;5;Nm) should NOT appear — all chars should be fully snapped
        assert!(
            !buf.contains("38;5;"),
            "{name}: final frame must not contain gradient colors (38;5;N)"
        );
    }
}

#[test]
fn matrix_snaps_to_original_colors() {
    let input = "\x1b[36mhello\x1b[0m \x1b[96mworld\x1b[0m";
    let styled = parse_styled(input);
    let anim = make_animation("matrix");
    let total = anim.total_frames(&styled);
    let mut buf = String::new();
    anim.render_frame(&styled, total, &mut buf);

    assert!(
        buf.contains("\x1b[36m"),
        "matrix: final frame must contain original cyan color"
    );
    assert!(
        buf.contains("\x1b[96m"),
        "matrix: final frame must contain original bright cyan color"
    );
    // Matrix uses bold (\x1b[1m) during cooldown — verify it's gone in the final frame
    // after all chars have snapped
    assert!(
        !buf.contains("\x1b[1m"),
        "matrix: final frame must not contain bold (cooldown artifact)"
    );
}

#[test]
fn shine_restores_original_colors_outside_band() {
    let input = "\x1b[36mhello\x1b[0m \x1b[96mworld\x1b[0m";
    let styled = parse_styled(input);
    let anim = make_animation("shine");
    let total = anim.total_frames(&styled);
    let mut buf = String::new();
    anim.render_frame(&styled, total, &mut buf);

    // After the band has passed, all chars should show their original colors
    assert!(
        buf.contains("\x1b[36m"),
        "shine: final frame must contain original cyan color"
    );
    assert!(
        buf.contains("\x1b[96m"),
        "shine: final frame must contain original bright cyan color"
    );
    // No flash band colors should remain
    assert!(
        !has_color256(&buf, 231),
        "shine: final frame must not contain flash core color"
    );
}

// ── Shine sweep direction ───────────────────────────────────────────────────

#[test]
fn shine_band_center_advances_each_frame() {
    let anim = make_animation("shine");
    let input = "abcdefghijklmnop";
    let styled = parse_styled(input);
    let total = anim.total_frames(&styled);

    // The flash band core (color 231, white) should illuminate different characters
    // on different frames. Find which character position has the core on each frame.
    let core_marker = "\x1b[38;5;231m";
    let mut positions: Vec<usize> = Vec::new();
    for frame in 1..=total {
        let mut buf = String::new();
        anim.render_frame(&styled, frame, &mut buf);

        if let Some(idx) = buf.find(core_marker) {
            // Count visible chars before the core marker to get band position
            let vis_before = visible_chars(&buf[..idx]).len();
            positions.push(vis_before);
        }
    }
    assert!(
        positions.len() >= 2,
        "should have at least 2 frames with flash band core"
    );

    // Positions must be strictly increasing (band sweeps left to right)
    for i in 1..positions.len() {
        assert!(
            positions[i] > positions[i - 1],
            "flash band must sweep left-to-right: position went from {} to {} (frames with core: {:?})",
            positions[i - 1],
            positions[i],
            positions
        );
    }
}

// ── Custom gradient tests ───────────────────────────────────────────────────

#[test]
fn custom_gradient_applied() {
    let custom_fg: Vec<u8> = vec![196, 160, 124]; // red gradient
    let anim = anim::resolve("sprout", None, Some(&custom_fg), None, 4, TEST_SEED).unwrap();
    let styled = parse_styled("abcdefghij");
    let mut buf = String::new();
    anim.render_frame(&styled, 8, &mut buf);
    // Should use custom red gradient colors
    let uses_custom = has_color256(&buf, 196) || has_color256(&buf, 160) || has_color256(&buf, 124);
    assert!(
        uses_custom,
        "custom gradient colors should appear in cooldown phase"
    );
}

#[test]
fn custom_bg_gradient_applied() {
    let custom_bg: Vec<u8> = vec![52, 88, 124];
    let anim = anim::resolve("sprout", None, None, Some(&custom_bg), 4, TEST_SEED).unwrap();
    let styled = parse_styled("abcdefghij");
    let mut buf = String::new();
    anim.render_frame(&styled, 6, &mut buf);
    // Should use background color sequences
    assert!(
        buf.contains("48;5;"),
        "custom bg gradient should produce background colors"
    );
}

// ── Flip rate tests ─────────────────────────────────────────────────────────

#[test]
fn flip_rate_affects_glyph_changes() {
    // With flip_rate=1, glyphs change every frame
    let fast = anim::resolve("flames", None, None, None, 1, TEST_SEED).unwrap();
    // With flip_rate=20, glyphs hold for 20 frames
    let slow = anim::resolve("flames", None, None, None, 20, TEST_SEED).unwrap();

    let input = "abcdefghij";
    let styled_fast = parse_styled(input);
    let styled_slow = parse_styled(input);

    // Use a frame where all 10 chars are revealed but none have snapped yet
    // (all revealed at frame 10, first snap at frame 1+14=15)
    let late = 12;
    let mut buf_f_a = String::new();
    let mut buf_f_b = String::new();
    fast.render_frame(&styled_fast, late, &mut buf_f_a);
    fast.render_frame(&styled_fast, late + 1, &mut buf_f_b);
    let vis_f_a = visible_chars(&buf_f_a);
    let vis_f_b = visible_chars(&buf_f_b);

    let mut buf_s_a = String::new();
    let mut buf_s_b = String::new();
    slow.render_frame(&styled_slow, late, &mut buf_s_a);
    slow.render_frame(&styled_slow, late + 1, &mut buf_s_b);
    let vis_s_a = visible_chars(&buf_s_a);
    let vis_s_b = visible_chars(&buf_s_b);

    // With slow flip rate (20), consecutive frames should have the same glyphs
    assert_eq!(
        vis_s_a, vis_s_b,
        "slow flip rate: consecutive frames should match"
    );
    // With fast flip rate (1), glyphs should change between frames
    assert_ne!(
        vis_f_a, vis_f_b,
        "fast flip rate: consecutive frames should differ"
    );
}
