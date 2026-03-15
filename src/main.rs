mod anim;
mod shell;
mod style;

use std::env;
use std::fs::OpenOptions;
use std::io::{self, IsTerminal, Read as IoRead, Write};
use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use style::parse_styled;

struct CliArgs {
    zsh: bool,
    duration: Option<u64>,
    gradient: Option<Vec<u8>>,
    positional: Vec<String>,
}

fn parse_cli_args() -> CliArgs {
    let mut args = env::args().skip(1);
    let mut zsh = false;
    let mut duration = None;
    let mut gradient = None;
    let mut positional = Vec::new();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--zsh" => {
                zsh = true;
            }
            "--duration" => {
                match args.next() {
                    None => {
                        eprintln!("error: --duration requires a value in milliseconds");
                        std::process::exit(1);
                    }
                    Some(val) => match val.parse::<u64>() {
                        Err(_) => {
                            eprintln!("error: --duration value must be a positive integer, got {:?}", val);
                            std::process::exit(1);
                        }
                        Ok(ms) => {
                            const MIN_MS: u64 = 50;
                            const MAX_MS: u64 = 10_000;
                            if ms < MIN_MS || ms > MAX_MS {
                                eprintln!("error: --duration must be between {} and {} ms", MIN_MS, MAX_MS);
                                std::process::exit(1);
                            }
                            duration = Some(ms);
                        }
                    },
                }
            }
            "--gradient" => {
                if let Some(val) = args.next() {
                    let parsed: Result<Vec<u8>, _> = val.split(',').map(|s| s.trim().parse::<u8>()).collect();
                    if let Ok(g) = parsed { if !g.is_empty() { gradient = Some(g); } }
                }
            }
            "-h" | "--help" | "help" => {
                eprintln!("Usage: zest [OPTIONS] [ANIMATION [COLOR]]");
                eprintln!();
                eprintln!("Animate your shell prompt into view on each redraw.");
                eprintln!();
                eprintln!("Animations:");
                for (name, desc) in anim::LIST {
                    let marker = if *name == anim::DEFAULT { " (default)" } else { "" };
                    eprintln!("  {:<14}{}{}", name, desc, marker);
                }
                eprintln!();
                eprintln!("Colors:");
                for (anim_name, colors) in anim::COLORS {
                    let first = colors[0];
                    let rest = &colors[1..];
                    let others: Vec<&str> = rest.to_vec();
                    eprintln!("  {}: {} (default){}", anim_name, first,
                        if others.is_empty() { String::new() } else { format!(", {}", others.join(", ")) });
                }
                eprintln!();
                eprintln!("Options:");
                eprintln!("      --duration <ms>  Total animation duration (50–10000, default 400)");
                eprintln!("      --gradient <c,...>  Custom gradient: comma-separated 256-color indices (0-255)");
                eprintln!("      --zsh            Wrap ANSI codes in %{{...%}} for zsh PROMPT");
                eprintln!("  -h, --help           Show this help");
            }
            _ => positional.push(arg),
        }
    }

    CliArgs { zsh, duration, gradient, positional }
}

fn read_input(is_piped: bool, rest: &[String]) -> String {
    if is_piped {
        let stdin = io::stdin();
        // Reading from pipe
        let mut input = String::new();
        stdin.lock().read_to_string(&mut input).unwrap();
        // Trim trailing newline
        if input.ends_with('\n') {
            input.pop();
            if input.ends_with('\r') {
                input.pop();
            }
        }
        input
    } else {
        // Fallback: CLI args
        rest.join(" ")
    }
}

/// Check if there are bytes waiting in the tty input buffer without consuming them.
fn tty_has_input(tty: &std::fs::File) -> bool {
    let mut count: libc::c_int = 0;
    unsafe { libc::ioctl(tty.as_raw_fd(), libc::FIONREAD, &mut count) };
    count > 0
}

fn main() {
    let cli = parse_cli_args();

    let stdin = io::stdin();
    let is_piped = !stdin.is_terminal();

    // Resolve animation from first positional arg, optionally consuming a color second arg
    let custom_gradient = cli.gradient.as_deref();
    let (animation, text_args) = if let Some(first) = cli.positional.first() {
        let maybe_color = cli.positional.get(1).map(String::as_str);
        if let Some(a) = anim::resolve(first, maybe_color, custom_gradient) {
            let consumed = if maybe_color.is_some() { 2 } else { 1 };
            (a, &cli.positional[consumed..])
        } else if let Some(a) = anim::resolve(first, None, custom_gradient) {
            // Valid animation name but unrecognized color — use default color, don't consume second arg
            (a, &cli.positional[1..])
        } else {
            // Unknown animation name — treat all positionals as text
            (anim::resolve(anim::DEFAULT, None, custom_gradient).unwrap(), cli.positional.as_slice())
        }
    } else {
        (anim::resolve(anim::DEFAULT, None, custom_gradient).unwrap(), cli.positional.as_slice())
    };

    let raw_input = if is_piped {
        read_input(true, &[])
    } else {
        read_input(false, text_args)
    };

    if raw_input.is_empty() {
        return;
    }

    let zsh = cli.zsh || shell::is_zsh();

    let styled = parse_styled(&raw_input);
    let total_frames = animation.total_frames(&styled);

    let mut frame_buf = String::with_capacity(styled.len() * 16);

    // Skip animation for very short prompts — too few chars for the sweep to read as intentional.
    const MIN_ANIMATION_CHARS: usize = 6;
    // Cap total animation duration; scale frame delay down for long prompts.
    const DEFAULT_DURATION_MS: u64 = 400;
    let target_duration = cli.duration.unwrap_or(DEFAULT_DURATION_MS);

    if styled.len() >= MIN_ANIMATION_CHARS {
        if let Ok(mut tty) = OpenOptions::new().read(true).write(true).open("/dev/tty") {
            let interrupted = Arc::new(AtomicBool::new(false));
            let flag = interrupted.clone();
            ctrlc::set_handler(move || flag.store(true, Ordering::Relaxed)).ok();

            let frame_delay = (target_duration / total_frames as u64).max(1);
            write!(tty, "\x1b[?25l").unwrap(); // hide cursor
            for frame in 1..=total_frames {
                if interrupted.load(Ordering::Relaxed) || tty_has_input(&tty) {
                    break;
                }
                frame_buf.clear();
                animation.render_frame(&styled, frame, &mut frame_buf);
                write!(tty, "\r{}", frame_buf).unwrap();
                tty.flush().unwrap();
                thread::sleep(Duration::from_millis(frame_delay));
            }
            // Return cursor to col 0 without erasing, keeping it hidden. The cursor restore is
            // emitted via stdout so it becomes visible only after the shell renders the prompt,
            // eliminating the brief flash of a visible cursor at col 0.
            write!(tty, "\r").unwrap();
            tty.flush().unwrap();
        }
    }

    // Final output to stdout (what the shell captures as the prompt).
    // \x1b[?25h (cursor restore) is appended here so it takes effect only after the shell
    // writes the prompt — preventing the cursor from briefly appearing at col 0.
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if zsh {
        // Wrap cursor restore in %{...%} so zsh doesn't count its bytes toward prompt width.
        write!(out, "{}%{{\x1b[?25h%}}", shell::wrap_ansi_for_zsh(&raw_input)).unwrap();
    } else {
        write!(out, "{}\x1b[?25h", raw_input).unwrap();
    }
    out.flush().unwrap();
}
