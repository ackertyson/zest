//! Fish shell integration tests.
//!
//! These tests validate the full fish prompt -> pipe -> zest -> terminal pipeline.
//! The PTY and fish-only tests verify piped I/O and fish prompt evaluation.
//! The tmux test is a smoke test for zest's most common deployment environment
//! (fish inside tmux) — it catches hangs, PTY detection failures, and process
//! lifecycle issues that only surface through tmux's terminal emulation layer.
//!
//! Tests are skipped if fish or tmux are not installed.
//!
//! Requires: fish (for all tests), tmux (for tmux tests only)

use std::os::fd::RawFd;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const PROMPT: &str = "\x1b[36m~/projects/zest\x1b[0m \x1b[96m❯ \x1b[0m";

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

fn has_command(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn tempdir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("zest-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

// ── Pipe-level integration (fish not required) ──────────────────────────────

#[test]
fn pipe_input_matches_direct() {
    // Spawn: sh -c "printf '<ANSI prompt>' | /path/to/zest --duration 200"
    // with a PTY as controlling terminal, verifying stdout matches direct invocation
    let zest_path = env!("CARGO_BIN_EXE_zest");

    let (master, slave) = open_pty();

    let mut cmd = Command::new("sh");
    cmd.args([
        "-c",
        &format!("printf '{}' | '{}' --duration 200", PROMPT, zest_path),
    ])
    .stdin(Stdio::null())
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

    let child = cmd.spawn().expect("failed to spawn sh|zest pipeline");
    unsafe { libc::close(slave) };

    let drain = drain_pty(master);
    let output = child.wait_with_output().unwrap();
    drain.join().unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let expected = format!("{PROMPT}\x1b[?25h");
    assert_eq!(
        stdout, expected,
        "piped input should produce same stdout as direct invocation"
    );
}

// ── Fish prompt integration ─────────────────────────────────────────────────

#[test]
fn fish_prompt_pipes_through_zest() {
    if !has_command("fish") {
        eprintln!("skipping: fish not installed");
        return;
    }

    let zest_path = env!("CARGO_BIN_EXE_zest");

    let tmp = tempdir();
    let functions_dir = tmp.join("functions");
    std::fs::create_dir_all(&functions_dir).unwrap();

    // 10 visible chars ("~/test ❯ ") — well above the 6-char animation minimum
    std::fs::write(
        functions_dir.join("fish_prompt.fish"),
        format!(
            r#"function fish_prompt
    begin
        set_color cyan
        echo -n "~/test"
        set_color normal
        set_color brcyan
        echo -n " ❯ "
        set_color normal
    end | '{zest_path}' --duration 50
end"#
        ),
    )
    .unwrap();

    let (master, slave) = open_pty();

    let mut cmd = Command::new("fish");
    cmd.args([
        "--no-config",
        "--init-command",
        &format!(
            "source {}",
            functions_dir.join("fish_prompt.fish").display()
        ),
        "-c",
        "echo __DONE__",
    ])
    .env("XDG_CONFIG_HOME", tmp.as_os_str())
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

    let child = cmd.spawn().expect("failed to spawn fish");
    unsafe { libc::close(slave) };

    let drain = drain_pty(master);
    let output = child.wait_with_output().unwrap();
    drain.join().unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("__DONE__"),
        "fish should execute command successfully, got: {:?}",
        stdout
    );
}

// ── Fish + tmux end-to-end ──────────────────────────────────────────────────

#[test]
fn fish_tmux_prompt_renders_correctly() {
    if !has_command("fish") || !has_command("tmux") {
        eprintln!("skipping: fish and/or tmux not installed");
        return;
    }

    let zest_path = env!("CARGO_BIN_EXE_zest");

    let tmp = tempdir();
    let functions_dir = tmp.join("functions");
    std::fs::create_dir_all(&functions_dir).unwrap();

    // 10 visible chars ("~/test ❯ ") — well above the 6-char animation minimum
    std::fs::write(
        functions_dir.join("fish_prompt.fish"),
        format!(
            r#"function fish_prompt
    begin
        set_color cyan
        echo -n "~/test"
        set_color normal
        set_color brcyan
        echo -n " ❯ "
        set_color normal
    end | '{zest_path}' --duration 100
end"#
        ),
    )
    .unwrap();

    let socket = tmp.join("tmux.sock");
    let socket_str = socket.to_str().unwrap();

    let mut tmux_start = Command::new("tmux");
    tmux_start
        .args([
            "-S",
            socket_str,
            "new-session",
            "-d",
            "-x",
            "120",
            "-y",
            "24",
            "fish",
            "--no-config",
            "--init-command",
            &format!(
                "source {}",
                functions_dir.join("fish_prompt.fish").display()
            ),
        ])
        .env("XDG_CONFIG_HOME", tmp.as_os_str())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let status = tmux_start.status().expect("failed to start tmux");
    assert!(status.success(), "tmux new-session failed");

    // Poll capture-pane until the prompt appears or timeout
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut captured = String::new();
    let mut found = false;

    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(200));

        let output = Command::new("tmux")
            .args(["-S", socket_str, "capture-pane", "-p"])
            .output()
            .expect("tmux capture-pane failed");

        captured = String::from_utf8_lossy(&output.stdout).to_string();
        if captured.contains("~/test") && captured.contains("❯") {
            found = true;
            break;
        }
    }

    let _ = Command::new("tmux")
        .args(["-S", socket_str, "kill-server"])
        .status();

    assert!(
        found,
        "prompt not found in tmux capture within 10s.\nCaptured:\n{captured}"
    );
}
