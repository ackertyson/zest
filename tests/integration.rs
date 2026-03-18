use std::io::{Read, Write};
use std::os::fd::RawFd;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

// A realistic prompt long enough to trigger animation, so we exercise the full code path:
// cursor hidden on tty → animation runs → cursor restored via stdout.
const PROMPT: &[u8] = b"\x1b[36m~/projects/zest\x1b[0m \x1b[96m\xe2\x9d\xaf \x1b[0m";

// A prompt short enough to skip animation (< 6 visible chars), exercising the
// no-animation path where the cursor is never hidden but restore is still emitted.
const SHORT_PROMPT: &[u8] = b"\x1b[36mhi\x1b[0m";

// ── Pty helpers ─────────────────────────────────────────────────────────────
//
// Each child gets its own pseudo-terminal so:
//   • /dev/tty opens successfully → the full animation path runs
//   • Animation output goes to the pty, not the real terminal
//   • Signals are isolated (child is a session leader in its own session)

/// Allocate a pty pair with a reasonable window size.
fn open_pty() -> (RawFd, RawFd) {
    let mut master: libc::c_int = 0;
    let mut slave: libc::c_int = 0;
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    ws.ws_col = 200;
    ws.ws_row = 24;
    let ret = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut ws,
        )
    };
    assert_eq!(ret, 0, "openpty failed");
    (master, slave)
}

/// Spawn zest with a pty as its controlling terminal.
/// Returns (child, pty_master_fd).
fn spawn_zest(input: &[u8], args: &[&str]) -> (std::process::Child, RawFd) {
    let (master, slave) = open_pty();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zest"));
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    unsafe {
        cmd.pre_exec(move || {
            libc::close(master); // child doesn't need the master side
            libc::setsid(); // new session → no controlling terminal yet
            libc::ioctl(slave, u64::from(libc::TIOCSCTTY), 0); // pty slave becomes controlling tty
            // Keep slave open through exec — if all slave fds close before the
            // exec'd process opens /dev/tty, Linux signals a hangup on the master
            // and the pty becomes permanently dead. The inherited fd is harmless
            // (cleaned up on process exit) and prevents the race.
            Ok(())
        });
    }

    let mut child = cmd.spawn().expect("failed to spawn zest");
    unsafe { libc::close(slave) }; // parent doesn't need slave

    child.stdin.as_mut().unwrap().write_all(input).unwrap();
    drop(child.stdin.take());

    (child, master)
}

/// Drain the pty master in a background thread so animation writes never block.
fn drain_pty(master: RawFd) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            let n = unsafe { libc::read(master, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
            if n <= 0 {
                break;
            }
        }
        unsafe { libc::close(master) };
    })
}

fn run_zest(input: &[u8], args: &[&str]) -> String {
    let (child, master) = spawn_zest(input, args);
    let drain = drain_pty(master);
    let output = child.wait_with_output().unwrap();
    drain.join().unwrap();
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

    // Wait for the animation to actually start by reading from the pty master.
    // The first frame write to the tty proves the animation loop (and signal
    // handlers) are live — much more reliable than a fixed sleep.
    // Set non-blocking so we can poll without hanging.
    unsafe {
        let flags = libc::fcntl(master, libc::F_GETFL);
        libc::fcntl(master, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
    let ready_deadline = Instant::now() + Duration::from_secs(5);
    let mut poll_buf = [0u8; 1];
    loop {
        let n = unsafe { libc::read(master, poll_buf.as_mut_ptr() as *mut libc::c_void, 1) };
        if n > 0 {
            break;
        }
        assert!(
            Instant::now() < ready_deadline,
            "animation did not start within 5 s"
        );
        thread::sleep(Duration::from_millis(5));
    }
    // Restore blocking mode for the drain thread.
    unsafe {
        let flags = libc::fcntl(master, libc::F_GETFL);
        libc::fcntl(master, libc::F_SETFL, flags & !libc::O_NONBLOCK);
    }
    // Now drain the rest of the pty in the background.
    let drain = drain_pty(master);

    // Drain stdout on a background thread so the pipe buffer never fills and
    // blocks the child process before it can act on the signal.
    let stdout_pipe = child.stdout.take().unwrap();
    let stdout_thread = thread::spawn(move || {
        let mut buf = Vec::new();
        std::io::BufReader::new(stdout_pipe)
            .read_to_end(&mut buf)
            .unwrap();
        buf
    });

    unsafe { libc::kill(child.id() as libc::pid_t, signum) };

    // The process must exit promptly after receiving the signal.
    // Worst-case wake latency: one frame sleep (10000 ms / ~30 frames ≈ 333 ms).
    let deadline = Instant::now() + Duration::from_millis(500);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "process did not exit within 500 ms after signal {signum}"
        );
        thread::sleep(Duration::from_millis(10));
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
