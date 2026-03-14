use std::io::Write;
use std::process::{Command, Stdio};

// A realistic prompt long enough to trigger animation, so we exercise the full code path:
// cursor hidden on tty → animation runs → cursor restored via stdout.
const PROMPT: &[u8] = b"\x1b[36m~/projects/zest\x1b[0m \x1b[96m\xe2\x9d\xaf \x1b[0m";

// A prompt short enough to skip animation (< 6 visible chars), exercising the
// no-animation path where the cursor is never hidden but restore is still emitted.
const SHORT_PROMPT: &[u8] = b"\x1b[36mhi\x1b[0m";

fn run_zest(input: &[u8], args: &[&str]) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_zest"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn zest");

    child.stdin.as_mut().unwrap().write_all(input).unwrap();
    drop(child.stdin.take()); // close stdin so process sees EOF
    let output = child.wait_with_output().unwrap();
    String::from_utf8(output.stdout).unwrap()
}

/// Fish mode: stdout must be the verbatim prompt followed by the cursor restore sequence.
/// The cursor restore is appended (not embedded in the prompt) so it fires only after the
/// shell renders the prompt — preventing the cursor from briefly appearing at col 0.
#[test]
fn fish_stdout_is_prompt_plus_cursor_restore() {
    let stdout = run_zest(PROMPT, &[]);
    let expected = format!("{}\x1b[?25h", String::from_utf8_lossy(PROMPT));
    assert_eq!(stdout, expected);
}

/// Zsh mode: ANSI sequences must be wrapped in %{...%} so zsh's width calculation
/// ignores their bytes (they are zero-width escape codes, not printed characters).
/// The cursor restore must also be wrapped — unwrapped it would shift the input
/// cursor 6 columns right, corrupting the command line display.
#[test]
fn zsh_stdout_wraps_sequences_and_cursor_restore() {
    let stdout = run_zest(PROMPT, &["--zsh"]);
    let expected = "%{\x1b[36m%}~/projects/zest%{\x1b[0m%} %{\x1b[96m%}❯ %{\x1b[0m%}%{\x1b[?25h%}";
    assert_eq!(stdout, expected);
}

/// Short prompt (< 6 visible chars): animation is skipped, cursor is never hidden.
/// The cursor restore must still be emitted — the code path is shared and the
/// sequence is idempotent (showing an already-visible cursor is a no-op).
#[test]
fn fish_short_prompt_still_emits_cursor_restore() {
    let stdout = run_zest(SHORT_PROMPT, &[]);
    let expected = format!("{}\x1b[?25h", String::from_utf8_lossy(SHORT_PROMPT));
    assert_eq!(stdout, expected);
}

#[test]
fn zsh_short_prompt_still_emits_wrapped_cursor_restore() {
    let stdout = run_zest(SHORT_PROMPT, &["--zsh"]);
    let expected = "%{\x1b[36m%}hi%{\x1b[0m%}%{\x1b[?25h%}";
    assert_eq!(stdout, expected);
}
