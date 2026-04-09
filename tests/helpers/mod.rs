use std::io::{Read, Write};
use std::os::fd::RawFd;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

// A realistic prompt long enough to trigger animation, so we exercise the full code path:
// cursor hidden on tty -> animation runs -> cursor restored via stdout.
pub const PROMPT: &[u8] = b"\x1b[36m~/projects/zest\x1b[0m \x1b[96m\xe2\x9d\xaf \x1b[0m";

// ── Pty helpers ─────────────────────────────────────────────────────────────
//
// Each child gets its own pseudo-terminal so:
//   - /dev/tty opens successfully -> the full animation path runs
//   - Animation output goes to the pty, not the real terminal
//   - Signals are isolated (child is a session leader in its own session)

/// Allocate a pty pair with a reasonable window size.
pub fn open_pty() -> (RawFd, RawFd) {
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
pub fn spawn_zest(input: &[u8], args: &[&str]) -> (std::process::Child, RawFd) {
    let (master, slave) = open_pty();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_zest"));
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    unsafe {
        cmd.pre_exec(move || {
            libc::close(master); // child doesn't need the master side
            libc::setsid(); // new session -> no controlling terminal yet
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
pub fn drain_pty(master: RawFd) -> thread::JoinHandle<()> {
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

pub fn run_zest(input: &[u8], args: &[&str]) -> String {
    let (child, master) = spawn_zest(input, args);
    let drain = drain_pty(master);
    let output = child.wait_with_output().unwrap();
    drain.join().unwrap();
    String::from_utf8(output.stdout).unwrap()
}

/// Wait for animation to start by polling for the first byte on the PTY master.
/// Returns after the first byte is read, proving the animation loop is live.
pub fn wait_for_animation_start(master: RawFd, timeout: Duration) {
    unsafe {
        let flags = libc::fcntl(master, libc::F_GETFL);
        libc::fcntl(master, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
    let deadline = Instant::now() + timeout;
    let mut poll_buf = [0u8; 1];
    loop {
        let n = unsafe { libc::read(master, poll_buf.as_mut_ptr() as *mut libc::c_void, 1) };
        if n > 0 {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "animation did not start within {:?}",
            timeout
        );
        thread::sleep(Duration::from_millis(5));
    }
    // Restore blocking mode for the drain thread.
    unsafe {
        let flags = libc::fcntl(master, libc::F_GETFL);
        libc::fcntl(master, libc::F_SETFL, flags & !libc::O_NONBLOCK);
    }
}

/// Drain stdout on a background thread so the pipe buffer never fills.
pub fn drain_stdout(child: &mut std::process::Child) -> thread::JoinHandle<Vec<u8>> {
    let stdout_pipe = child.stdout.take().unwrap();
    thread::spawn(move || {
        let mut buf = Vec::new();
        std::io::BufReader::new(stdout_pipe)
            .read_to_end(&mut buf)
            .unwrap();
        buf
    })
}
