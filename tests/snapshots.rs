//! Visual regression tests via render_frame() snapshots.
//!
//! Each animation+color combination is rendered for every frame and the output
//! is stored as a readable escaped-string snapshot file. On subsequent runs,
//! the rendered output is compared against the stored snapshot.
//!
//! To regenerate snapshots: ZEST_UPDATE_SNAPSHOTS=1 cargo test --test snapshots

use std::path::PathBuf;

use zest::anim::{self, Animation};
use zest::style::parse_styled;

const SNAPSHOT_INPUT: &str = "\x1b[36m~/projects/zest\x1b[0m \x1b[96m❯ \x1b[0m";

fn snapshot_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots")
}

fn should_update() -> bool {
    std::env::var("ZEST_UPDATE_SNAPSHOTS").is_ok()
}

/// Escape a rendered frame string into a readable, diffable representation.
/// Control chars and non-ASCII are escaped; printable ASCII is kept verbatim.
fn escape_frame(frame: &str) -> String {
    let mut out = String::new();
    for ch in frame.chars() {
        match ch {
            '\x1b' => out.push_str("\\x1b"),
            c if c.is_ascii_graphic() || c == ' ' => out.push(c),
            c => {
                for b in c.to_string().as_bytes() {
                    out.push_str(&format!("\\x{b:02x}"));
                }
            }
        }
    }
    out
}

fn render_snapshot(anim: &dyn Animation, input: &str) -> String {
    let styled = parse_styled(input);
    let total = anim.total_frames(&styled);
    let mut lines = Vec::with_capacity(total);
    for frame in 1..=total {
        let mut buf = String::new();
        anim.render_frame(&styled, frame, &mut buf);
        lines.push(format!("frame {frame}: {}", escape_frame(&buf)));
    }
    lines.join("\n") + "\n"
}

fn snapshot_test(name: &str, color: Option<&str>) {
    let anim = anim::resolve(name, color, None, None, 4)
        .unwrap_or_else(|| panic!("failed to resolve {name}/{color:?}"));

    let actual = render_snapshot(anim.as_ref(), SNAPSHOT_INPUT);
    let file_name = match color {
        Some(c) => format!("{name}_{c}.snap"),
        None => format!("{name}.snap"),
    };
    let path = snapshot_dir().join(&file_name);

    if should_update() {
        std::fs::write(&path, &actual)
            .unwrap_or_else(|e| panic!("failed to write snapshot {file_name}: {e}"));
        return;
    }

    let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "snapshot {file_name} not found: {e}\n\
             Run with ZEST_UPDATE_SNAPSHOTS=1 to generate"
        )
    });

    if actual != expected {
        // Find first differing frame for a helpful error message
        let actual_lines: Vec<&str> = actual.lines().collect();
        let expected_lines: Vec<&str> = expected.lines().collect();
        let mut diff_frame = None;
        for (i, (a, e)) in actual_lines.iter().zip(expected_lines.iter()).enumerate() {
            if a != e {
                diff_frame = Some((i + 1, *a, *e));
                break;
            }
        }
        if let Some((line, actual_line, expected_line)) = diff_frame {
            panic!(
                "snapshot mismatch in {file_name} at line {line}:\n\
                 expected: {expected_line}\n\
                 actual:   {actual_line}\n\n\
                 Run with ZEST_UPDATE_SNAPSHOTS=1 to update"
            );
        } else {
            panic!(
                "snapshot mismatch in {file_name}: different number of frames \
                 (expected {}, got {})\n\
                 Run with ZEST_UPDATE_SNAPSHOTS=1 to update",
                expected_lines.len(),
                actual_lines.len()
            );
        }
    }
}

// ── Per-animation snapshot tests ────────────────────────────────────────────

#[test]
fn snapshot_sprout() {
    snapshot_test("sprout", None);
}

#[test]
fn snapshot_flames() {
    snapshot_test("flames", None);
}

#[test]
fn snapshot_matrix() {
    snapshot_test("matrix", None);
}

#[test]
fn snapshot_scan() {
    snapshot_test("scan", None);
}

#[test]
fn snapshot_shine() {
    snapshot_test("shine", None);
}
