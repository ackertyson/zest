mod anim;
mod shell;
mod style;

use std::env;
use std::fs::OpenOptions;
use std::io::{self, IsTerminal, Read as IoRead, Write};
use std::thread;
use std::time::Duration;

use style::parse_styled;

struct CliArgs {
    zsh: bool,
    positional: Vec<String>,
}

fn parse_cli_args() -> CliArgs {
    let mut args = env::args().skip(1);
    let mut zsh = false;
    let mut positional = Vec::new();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--zsh" => {
                zsh = true;
            }
            "-h" | "--help" => {
                eprintln!("Usage: zest [ANIMATION] [OPTIONS]");
                eprintln!();
                eprintln!("Animates a colorized prompt into view.");
                eprintln!();
                eprintln!("Arguments:");
                eprintln!("  [ANIMATION]  Animation style: green-flash, flames (default: {})", anim::DEFAULT);
                eprintln!();
                eprintln!("Options:");
                eprintln!("      --zsh    Wrap ANSI codes in %{{...%}} for zsh PROMPT");
                eprintln!("  -h, --help   Show this help");
            }
            _ => positional.push(arg),
        }
    }

    CliArgs { zsh, positional }
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

    let stdin = io::stdin();
    let is_piped = !stdin.is_terminal();

    // Resolve animation from first positional arg
    let (animation, text_args) = if let Some(first) = cli.positional.first() {
        if let Some(a) = anim::resolve(first) {
            (a, &cli.positional[1..])
        } else {
            (anim::resolve(anim::DEFAULT).unwrap(), cli.positional.as_slice())
        }
    } else {
        (anim::resolve(anim::DEFAULT).unwrap(), cli.positional.as_slice())
    };

    let raw_input = if is_piped {
        read_input(&[])
    } else {
        read_input(text_args)
    };

    if raw_input.is_empty() {
        return;
    }

    let zsh = cli.zsh || shell::is_zsh();

    let styled = parse_styled(&raw_input);
    let total_frames = animation.total_frames(&styled);

    let mut frame_buf = String::with_capacity(styled.len() * 16);

    // Write animation frames directly to the terminal (/dev/tty) so they're
    // visible even when stdout is captured (e.g. by fish's fish_prompt).
    // The final prompt goes to stdout for fish to use.
    if let Ok(mut tty) = OpenOptions::new().write(true).open("/dev/tty") {
        write!(tty, "\x1b[?25l").unwrap(); // hide cursor
        for frame in 1..=total_frames {
            frame_buf.clear();
            animation.render_frame(&styled, frame, &mut frame_buf);
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
        write!(out, "{}", shell::wrap_ansi_for_zsh(&raw_input)).unwrap();
    } else {
        write!(out, "{}", raw_input).unwrap();
    }
    out.flush().unwrap();
}
