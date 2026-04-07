//! Stability tests for edge cases that could break animation in real shell usage.
//!
//! These tests exercise failure modes that occur in production: terminal resizes,
//! rapid re-prompts, broken pipes, terminal disconnects, and signal edge cases.

mod helpers;

use std::time::{Duration, Instant};

use helpers::*;

// ── SIGWINCH during animation ───────────────────────────────────────────────

#[test]
fn sigwinch_during_animation_no_crash() {
    let (mut child, master) = spawn_zest(PROMPT, &["--duration", "10000"]);

    wait_for_animation_start(master, Duration::from_secs(5));

    // Send SIGWINCH (terminal resize) — should not crash or hang
    unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGWINCH) };

    // Give the animation a moment to process the signal
    std::thread::sleep(Duration::from_millis(50));

    // Now send SIGINT to cleanly stop the animation
    let drain = drain_pty(master);
    let stdout_thread = drain_stdout(&mut child);

    unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGINT) };

    let deadline = Instant::now() + Duration::from_millis(500);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "process did not exit within 500 ms after SIGINT (post-SIGWINCH)"
        );
        std::thread::sleep(Duration::from_millis(10));
    }

    let stdout = String::from_utf8(stdout_thread.join().unwrap()).unwrap();
    drain.join().unwrap();
    let expected = format!("{}\x1b[?25h", String::from_utf8_lossy(PROMPT));
    assert_eq!(
        stdout, expected,
        "stdout must be correct after SIGWINCH + SIGINT"
    );
}

// ── Rapid reprompt ──────────────────────────────────────────────────────────

#[test]
fn rapid_reprompt_all_correct() {
    // Simulate fish re-prompting 10 times in rapid succession with short animations
    let expected = format!("{}\x1b[?25h", String::from_utf8_lossy(PROMPT));

    for i in 0..10 {
        let stdout = run_zest(PROMPT, &["--duration", "50"]);
        assert_eq!(
            stdout, expected,
            "rapid reprompt iteration {i}: stdout mismatch"
        );
    }
}

// ── stdin closed early ──────────────────────────────────────────────────────

#[test]
fn stdin_closed_early_exits_silently() {
    // Send empty input — should exit with empty output (no crash, no hang)
    let stdout = run_zest(b"", &[]);
    assert!(
        stdout.is_empty(),
        "empty input should produce empty output, got: {stdout:?}"
    );
}

// ── stdout broken pipe ─────────────────────────────────────────────────────

#[test]
fn stdout_broken_pipe_no_panic() {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    let (master, slave) = open_pty();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zest"));
    cmd.args(["--duration", "50"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    unsafe {
        cmd.pre_exec(move || {
            libc::close(master);
            libc::setsid();
            libc::ioctl(slave, u64::from(libc::TIOCSCTTY), 0);
            Ok(())
        });
    }

    let mut child = cmd.spawn().expect("failed to spawn zest");
    unsafe { libc::close(slave) };

    // Write input
    {
        use std::io::Write;
        child.stdin.as_mut().unwrap().write_all(PROMPT).unwrap();
        drop(child.stdin.take());
    }

    // Close stdout immediately to create a broken pipe scenario
    drop(child.stdout.take());

    let drain = drain_pty(master);

    // The process should still exit (not hang forever)
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "process did not exit within 5 s after broken stdout pipe"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    drain.join().unwrap();
}

// ── PTY master closed ───────────────────────────────────────────────────────

#[test]
fn pty_master_closed_triggers_exit() {
    let (mut child, master) = spawn_zest(PROMPT, &["--duration", "10000"]);

    wait_for_animation_start(master, Duration::from_secs(5));

    // Drain stdout in background so the pipe doesn't block
    let stdout_thread = drain_stdout(&mut child);

    // Close the PTY master — simulates terminal disconnect.
    // This causes writes to /dev/tty to fail. The process must exit cleanly
    // (not panic) and still write the prompt to stdout.
    unsafe { libc::close(master) };

    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "process did not exit within 5 s after PTY master closed"
        );
        std::thread::sleep(Duration::from_millis(50));
    }

    let stdout = String::from_utf8(stdout_thread.join().unwrap()).unwrap();
    let expected = format!("{}\x1b[?25h", String::from_utf8_lossy(PROMPT));
    assert_eq!(
        stdout, expected,
        "stdout must be correct after PTY disconnect — prompt must always be emitted"
    );
}

// ── Signal during select() ──────────────────────────────────────────────────

#[test]
fn signal_during_select_wakes_immediately() {
    // Use a very long frame delay so select() is definitely blocked when we signal
    let (mut child, master) = spawn_zest(PROMPT, &["--duration", "10000"]);

    wait_for_animation_start(master, Duration::from_secs(5));

    let drain = drain_pty(master);
    let stdout_thread = drain_stdout(&mut child);

    // Wait a bit to ensure we're in the select() between frames
    std::thread::sleep(Duration::from_millis(100));

    let signal_time = Instant::now();
    unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGTERM) };

    // Should wake within 500ms (not wait for the full ~333ms frame delay to expire)
    let deadline = Instant::now() + Duration::from_millis(500);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "process did not wake from select() within 500 ms"
        );
        std::thread::sleep(Duration::from_millis(5));
    }

    let exit_latency = signal_time.elapsed();
    // select() should be interrupted, not block for the full frame delay
    assert!(
        exit_latency < Duration::from_millis(500),
        "exit latency after signal was {:?} — select() should wake immediately",
        exit_latency
    );

    let _stdout = stdout_thread.join().unwrap();
    drain.join().unwrap();
}

// ── Multiple rapid signals ──────────────────────────────────────────────────

#[test]
fn multiple_rapid_signals_no_panic() {
    let (mut child, master) = spawn_zest(PROMPT, &["--duration", "10000"]);

    wait_for_animation_start(master, Duration::from_secs(5));

    let drain = drain_pty(master);
    let stdout_thread = drain_stdout(&mut child);

    // Send SIGINT twice in rapid succession
    let pid = child.id() as libc::pid_t;
    unsafe {
        libc::kill(pid, libc::SIGINT);
        libc::kill(pid, libc::SIGINT);
    }

    let deadline = Instant::now() + Duration::from_millis(500);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "process did not exit within 500 ms after double SIGINT"
        );
        std::thread::sleep(Duration::from_millis(10));
    }

    let stdout = String::from_utf8(stdout_thread.join().unwrap()).unwrap();
    drain.join().unwrap();
    let expected = format!("{}\x1b[?25h", String::from_utf8_lossy(PROMPT));
    assert_eq!(
        stdout, expected,
        "stdout must be correct after double SIGINT"
    );
}

// ── Startup latency ─────────────────────────────────────────────────────────

#[test]
fn startup_latency_under_10ms() {
    use std::process::{Command, Stdio};

    // Measure end-to-end: spawn zest with piped input, time until it exits.
    // Uses --duration 50 (minimum) with a short prompt and no PTY, so zest
    // reads stdin, discovers there's no controlling terminal, skips animation,
    // and writes stdout. This measures the real overhead a shell pays for
    // fork+exec+parse+output — the same path as `printf ... | zest`.
    const RUNS: usize = 21;
    let mut times = Vec::with_capacity(RUNS);

    for _ in 0..RUNS {
        let start = Instant::now();
        let mut child = Command::new(env!("CARGO_BIN_EXE_zest"))
            .args(["--duration", "50"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to spawn zest");

        {
            use std::io::Write;
            child.stdin.as_mut().unwrap().write_all(PROMPT).unwrap();
            drop(child.stdin.take());
        }

        let output = child.wait_with_output().unwrap();
        let elapsed = start.elapsed();

        assert!(output.status.success(), "zest exited with error");
        assert!(!output.stdout.is_empty(), "zest produced no output");

        times.push(elapsed);
    }

    times.sort();
    let median = times[RUNS / 2];
    eprintln!("startup latency (median of {RUNS}): {median:?}");

    assert!(
        median < Duration::from_millis(10),
        "startup latency median was {:?} — should be under 10ms",
        median
    );
}

// ── First-paint latency (PTY path) ───────────────────────────────────────────

#[test]
fn first_paint_latency_under_20ms() {
    // Measures spawn-to-first-TTY-byte with a real PTY — the full animation path
    // including open("/dev/tty"), signal handlers, cursor hide, and first frame.
    // Uses select() instead of polling for precise timing.
    const RUNS: usize = 11;
    let mut times = Vec::with_capacity(RUNS);

    for _ in 0..RUNS {
        let start = Instant::now();
        let (mut child, master) = spawn_zest(PROMPT, &["--duration", "10000"]);

        // Block on select() until the first byte arrives on the PTY master.
        unsafe {
            let mut read_fds: libc::fd_set = std::mem::zeroed();
            libc::FD_SET(master, &mut read_fds);
            let mut tv = libc::timeval {
                tv_sec: 5,
                tv_usec: 0,
            };
            let ready = libc::select(
                master + 1,
                &mut read_fds,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut tv,
            );
            assert!(ready > 0, "no TTY output within 5 s");
        }
        let elapsed = start.elapsed();
        times.push(elapsed);

        // Clean up
        let drain = drain_pty(master);
        let stdout_thread = drain_stdout(&mut child);
        unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGINT) };
        child.wait().unwrap();
        let _ = stdout_thread.join();
        drain.join().unwrap();
    }

    times.sort();
    let median = times[RUNS / 2];
    eprintln!("first paint latency (median of {RUNS}): {median:?}");

    // Median is typically ~2ms. 20ms threshold gives 10x headroom for CI load
    // while catching regressions that add blocking work before the first TTY write.
    assert!(
        median < Duration::from_millis(20),
        "first paint latency median was {:?} — should be under 20ms",
        median
    );
}

// ── All animations complete without crash ───────────────────────────────────

#[test]
fn all_animations_complete_with_correct_stdout() {
    let expected = format!("{}\x1b[?25h", String::from_utf8_lossy(PROMPT));
    for name in ["sprout", "flames", "matrix", "scan", "shine"] {
        let stdout = run_zest(PROMPT, &[name, "--duration", "100"]);
        assert_eq!(stdout, expected, "animation {name}: stdout mismatch");
    }
}
