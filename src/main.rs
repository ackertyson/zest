mod anim;
mod shell;
mod style;

use std::env;
use std::fs::OpenOptions;
use std::io::{self, IsTerminal, Read as IoRead, Write};
use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use style::parse_styled;

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

unsafe extern "C" fn handle_signal(_: libc::c_int) {
    INTERRUPTED.store(true, Ordering::Relaxed);
}

struct CliArgs {
    zsh: bool,
    duration: Option<u64>,
    flip_rate: Option<usize>,
    gradient: Option<(Option<Vec<u8>>, Option<Vec<u8>>)>,
    positional: Vec<String>,
}

/// Parse a `--gradient` value into `(fg, bg)` lists.
/// Accepts `"fg1,fg2"`, `"fg1,fg2:bg1,bg2"`, or `":bg1,bg2"`.
/// Returns `None` if both sides are absent or unparseable.
fn parse_gradient(val: &str) -> Option<(Option<Vec<u8>>, Option<Vec<u8>>)> {
    let (fg_part, bg_part) = match val.split_once(':') {
        Some((f, b)) => (f, Some(b)),
        None         => (val, None),
    };
    let parse_list = |s: &str| -> Option<Vec<u8>> {
        let v: Result<Vec<u8>, _> = s.split(',').map(|x| x.trim().parse::<u8>()).collect();
        v.ok().filter(|v| !v.is_empty())
    };
    let fg = parse_list(fg_part);
    let bg = bg_part.and_then(parse_list);
    if fg.is_some() || bg.is_some() { Some((fg, bg)) } else { None }
}

const DURATION_MIN_MS: u64 = 50;
const DURATION_MAX_MS: u64 = 10_000;

const FLIP_RATE_MIN: usize = 1;
const FLIP_RATE_MAX: usize = 20;

fn parse_flip_rate(val: &str) -> Result<usize, String> {
    let n = val.parse::<usize>()
        .map_err(|_| format!("error: --flip-rate value must be a positive integer, got {:?}", val))?;
    if n < FLIP_RATE_MIN || n > FLIP_RATE_MAX {
        return Err(format!("error: --flip-rate must be between {} and {}", FLIP_RATE_MIN, FLIP_RATE_MAX));
    }
    Ok(n)
}

/// Validate a `--duration` string. Returns `Ok(ms)` or `Err(message)`.
fn parse_duration(val: &str) -> Result<u64, String> {
    let ms = val.parse::<u64>()
        .map_err(|_| format!("error: --duration value must be a positive integer, got {:?}", val))?;
    if ms < DURATION_MIN_MS || ms > DURATION_MAX_MS {
        return Err(format!("error: --duration must be between {} and {} ms", DURATION_MIN_MS, DURATION_MAX_MS));
    }
    Ok(ms)
}

fn parse_cli_args() -> CliArgs {
    let mut args = env::args().skip(1);
    let mut zsh = false;
    let mut duration = None;
    let mut flip_rate = None;
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
                    Some(val) => match parse_duration(&val) {
                        Ok(ms) => duration = Some(ms),
                        Err(msg) => { eprintln!("{}", msg); std::process::exit(1); }
                    },
                }
            }
            "--flip-rate" => {
                match args.next() {
                    None => { eprintln!("error: --flip-rate requires a value"); std::process::exit(1); }
                    Some(val) => match parse_flip_rate(&val) {
                        Ok(n) => flip_rate = Some(n),
                        Err(msg) => { eprintln!("{}", msg); std::process::exit(1); }
                    },
                }
            }
            "--gradient" => {
                if let Some(val) = args.next() {
                    gradient = parse_gradient(&val);
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
                eprintln!("      --flip-rate <n>  Glyph change rate for flames/matrix (1–20, default 4)");
                eprintln!("      --gradient <fg[:bg]>  Custom gradient: comma-separated 256-color indices; optional :bg list for background");
                eprintln!("      --zsh            Wrap ANSI codes in %{{...%}} for zsh PROMPT");
                eprintln!("  -h, --help           Show this help");
            }
            _ => positional.push(arg),
        }
    }

    CliArgs { zsh, duration, flip_rate, gradient, positional }
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
    let (custom_fg, custom_bg) = match &cli.gradient {
        Some((fg, bg)) => (fg.as_deref(), bg.as_deref()),
        None => (None, None),
    };
    let flip_rate = cli.flip_rate.unwrap_or(4);
    let (animation, text_args) = if let Some(first) = cli.positional.first() {
        let maybe_color = cli.positional.get(1).map(String::as_str);
        if let Some(a) = anim::resolve(first, maybe_color, custom_fg, custom_bg, flip_rate) {
            let consumed = if maybe_color.is_some() { 2 } else { 1 };
            (a, &cli.positional[consumed..])
        } else if let Some(a) = anim::resolve(first, None, custom_fg, custom_bg, flip_rate) {
            // Valid animation name but unrecognized color — use default color, don't consume second arg
            (a, &cli.positional[1..])
        } else {
            // Unknown animation name — treat all positionals as text
            (anim::resolve(anim::DEFAULT, None, custom_fg, custom_bg, flip_rate).unwrap(), cli.positional.as_slice())
        }
    } else {
        (anim::resolve(anim::DEFAULT, None, custom_fg, custom_bg, flip_rate).unwrap(), cli.positional.as_slice())
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
            unsafe {
                libc::signal(libc::SIGINT,  handle_signal as libc::sighandler_t);
                libc::signal(libc::SIGTERM, handle_signal as libc::sighandler_t);
                libc::signal(libc::SIGHUP,  handle_signal as libc::sighandler_t);
            }

            let frame_delay = (target_duration / total_frames as u64).max(1);
            write!(tty, "\x1b[?25l").unwrap(); // hide cursor
            for frame in 1..=total_frames {
                if INTERRUPTED.load(Ordering::Relaxed) || tty_has_input(&tty) {
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_gradient ───────────────────────────────────────────────────────

    #[test]
    fn gradient_fg_only() {
        assert_eq!(parse_gradient("226,220,214"), Some((Some(vec![226, 220, 214]), None)));
    }

    #[test]
    fn gradient_fg_and_bg() {
        assert_eq!(parse_gradient("226,220:52,88"), Some((Some(vec![226, 220]), Some(vec![52, 88]))));
    }

    #[test]
    fn gradient_bg_only() {
        assert_eq!(parse_gradient(":52,22,18"), Some((None, Some(vec![52, 22, 18]))));
    }

    #[test]
    fn gradient_fg_empty_bg() {
        // Colon present but BG side is empty — BG treated as absent
        assert_eq!(parse_gradient("196:"), Some((Some(vec![196]), None)));
    }

    #[test]
    fn gradient_single_value() {
        assert_eq!(parse_gradient("128"), Some((Some(vec![128]), None)));
    }

    #[test]
    fn gradient_boundary_values() {
        assert_eq!(parse_gradient("0"), Some((Some(vec![0]), None)));
        assert_eq!(parse_gradient("255"), Some((Some(vec![255]), None)));
    }

    #[test]
    fn gradient_out_of_range_rejected() {
        // 256 overflows u8
        assert_eq!(parse_gradient("256"), None);
    }

    #[test]
    fn gradient_non_numeric_rejected() {
        assert_eq!(parse_gradient("red"), None);
        assert_eq!(parse_gradient("12,abc,34"), None);
    }

    #[test]
    fn gradient_both_sides_empty_rejected() {
        assert_eq!(parse_gradient(":"), None);
    }

    #[test]
    fn gradient_empty_string_rejected() {
        assert_eq!(parse_gradient(""), None);
    }

    #[test]
    fn gradient_whitespace_trimmed() {
        // Spaces around commas are accepted
        assert_eq!(parse_gradient("226, 220, 214"), Some((Some(vec![226, 220, 214]), None)));
        assert_eq!(parse_gradient("10, 20 : 30, 40"), Some((Some(vec![10, 20]), Some(vec![30, 40]))));
    }

    #[test]
    fn gradient_bg_only_invalid_rejected() {
        // BG side has invalid value → BG is None; FG is also absent → None overall
        assert_eq!(parse_gradient(":xyz"), None);
    }

    #[test]
    fn gradient_fg_invalid_bg_valid() {
        // FG side invalid → FG is None; BG valid → Some((None, Some([52])))
        assert_eq!(parse_gradient("xyz:52"), Some((None, Some(vec![52]))));
    }

    // ── parse_duration ───────────────────────────────────────────────────────

    #[test]
    fn duration_valid_range() {
        assert_eq!(parse_duration("50"), Ok(50));
        assert_eq!(parse_duration("400"), Ok(400));
        assert_eq!(parse_duration("10000"), Ok(10_000));
    }

    #[test]
    fn duration_below_minimum_rejected() {
        assert!(parse_duration("49").is_err());
        assert!(parse_duration("0").is_err());
    }

    #[test]
    fn duration_above_maximum_rejected() {
        assert!(parse_duration("10001").is_err());
    }

    #[test]
    fn duration_non_integer_rejected() {
        assert!(parse_duration("fast").is_err());
        assert!(parse_duration("1.5").is_err());
        assert!(parse_duration("").is_err());
        assert!(parse_duration("-100").is_err());
    }

    #[test]
    fn duration_error_messages() {
        let msg = parse_duration("abc").unwrap_err();
        assert!(msg.contains("positive integer"), "unexpected: {msg}");
        let msg = parse_duration("49").unwrap_err();
        assert!(msg.contains("between"), "unexpected: {msg}");
    }
}
