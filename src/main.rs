mod anim;
mod style;

use std::env;
use std::fs::OpenOptions;
use std::io::{self, IsTerminal, Read as IoRead, Write};
use std::thread;
use std::time::Duration;

use style::parse_styled;

struct CliArgs {
    animation: String,
    zsh: bool,
    rest: Vec<String>,
}

fn parse_cli_args() -> CliArgs {
    let mut args = env::args().skip(1);
    let mut animation = anim::DEFAULT.to_string();
    let mut zsh = false;
    let mut rest = Vec::new();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-a" | "--animation" => {
                if let Some(name) = args.next() {
                    animation = name;
                }
            }
            "--zsh" => {
                zsh = true;
            }
            "-h" | "--help" => {
                eprintln!("Usage: zest [OPTIONS] [TEXT...]");
                eprintln!();
                eprintln!("Animates a colorized prompt into view.");
                eprintln!();
                eprintln!("Options:");
                eprintln!("  -a, --animation <name>  Animation style (default: {})", anim::DEFAULT);
                eprintln!("      --zsh               Wrap ANSI codes in %{{...%}} for zsh PROMPT");
                eprintln!("  -h, --help              Show this help");
            }
            _ => rest.push(arg),
        }
    }

    CliArgs { animation, zsh, rest }
}

/// Wraps every ANSI escape sequence in `%{...%}` so zsh counts prompt width correctly.
fn wrap_ansi_for_zsh(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 32);
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if chars.peek() == Some(&'[') {
                // Collect the full CSI sequence (ends at first byte in 0x40–0x7E)
                let mut seq = String::from("\x1b");
                seq.push(chars.next().unwrap()); // '['
                for c in chars.by_ref() {
                    let done = ('\x40'..='\x7e').contains(&c);
                    seq.push(c);
                    if done {
                        break;
                    }
                }
                out.push_str("%{");
                out.push_str(&seq);
                out.push_str("%}");
            } else {
                out.push(ch);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn read_input(rest: &[String]) -> String {
    let stdin = io::stdin();
    if !stdin.is_terminal() {
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

fn main() {
    let cli = parse_cli_args();

    let animation = match anim::resolve(&cli.animation) {
        Some(a) => a,
        None => {
            eprintln!("zest: unknown animation '{}', falling back to '{}'", cli.animation, anim::DEFAULT);
            anim::resolve(anim::DEFAULT).unwrap()
        }
    };

    let raw_input = read_input(&cli.rest);

    if raw_input.is_empty() {
        return;
    }

    let zsh = cli.zsh || env::var("ZSH_VERSION").is_ok();

    let styled = parse_styled(&raw_input);
    let n = styled.len();
    let total_frames = animation.total_frames(n);

    let mut frame_buf = String::with_capacity(n * 16);

    // Write animation frames directly to the terminal (/dev/tty) so they're
    // visible even when stdout is captured (e.g. by fish's fish_prompt).
    // The final prompt goes to stdout for fish to use.
    if let Ok(mut tty) = OpenOptions::new().write(true).open("/dev/tty") {
        write!(tty, "\x1b[?25l").unwrap(); // hide cursor
        for frame in 1..=total_frames {
            frame_buf.clear();
            animation.render_frame(&styled, n, frame, &mut frame_buf);
            write!(tty, "\r{}", frame_buf).unwrap();
            tty.flush().unwrap();
            thread::sleep(Duration::from_millis(animation.frame_delay_ms()));
        }
        // Clear animation line before final output
        write!(tty, "\r\x1b[K").unwrap();
        write!(tty, "\x1b[?25h").unwrap(); // restore cursor
        tty.flush().unwrap();
    }

    // Final output to stdout (what the shell captures as the prompt)
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if zsh {
        write!(out, "{}", wrap_ansi_for_zsh(&raw_input)).unwrap();
    } else {
        write!(out, "{}", raw_input).unwrap();
    }
    out.flush().unwrap();
}
