mod helpers;

use std::time::{Duration, Instant};

use helpers::*;

// A prompt short enough to skip animation (< 6 visible chars), exercising the
// no-animation path where the cursor is never hidden but restore is still emitted.
const SHORT_PROMPT: &[u8] = b"\x1b[36mhi\x1b[0m";

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

// ── Signal interrupt tests ────────────────────────────────────────────────────
//
// Each test spawns zest with --duration 10000 (10 s) so the animation is still
// running when the signal arrives. The child runs in its own pty and session, so
// signals are fully isolated from the test runner. We wait for pty output before
// signalling, proving the animation loop (and signal handlers) are live.
// Assertions:
//   1. The process exits within 500 ms of the signal.
//   2. stdout is the verbatim prompt followed by \x1b[?25h — the final write is
//      unconditional, so it must fire regardless of how early the loop was cut.

fn assert_signal_exits_cleanly(signum: libc::c_int) {
    let (mut child, master) = spawn_zest(PROMPT, &["--duration", "10000"]);

    wait_for_animation_start(master, Duration::from_secs(5));

    // Now drain the rest of the pty in the background.
    let drain = drain_pty(master);

    let stdout_thread = drain_stdout(&mut child);

    unsafe { libc::kill(child.id() as libc::pid_t, signum) };

    // The process must exit promptly after receiving the signal.
    let deadline = Instant::now() + Duration::from_millis(500);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "process did not exit within 500 ms after signal {signum}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }

    let stdout = String::from_utf8(stdout_thread.join().unwrap()).unwrap();
    drain.join().unwrap();
    let expected = format!("{}\x1b[?25h", String::from_utf8_lossy(PROMPT));
    assert_eq!(
        stdout, expected,
        "stdout mismatch after signal {signum}: prompt+cursor-restore must always be emitted"
    );
}

#[test]
fn sigint_exits_cleanly() {
    assert_signal_exits_cleanly(libc::SIGINT);
}

#[test]
fn sigterm_exits_cleanly() {
    assert_signal_exits_cleanly(libc::SIGTERM);
}

#[test]
fn sighup_exits_cleanly() {
    assert_signal_exits_cleanly(libc::SIGHUP);
}
