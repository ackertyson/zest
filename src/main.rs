use std::env;
use std::fmt::Write as FmtWrite;
use std::fs::OpenOptions;
use std::io::{self, IsTerminal, Read as IoRead, Write};
use std::thread;
use std::time::Duration;

const SPINNERS: [char; 4] = ['-', '\\', '|', '/'];
const FRAME_DELAY_MS: u64 = 10;
const COOLDOWN_FRAMES: usize = 12;

// 256-color gradient from hot (bright greenish-white) to resting dark green.
// Each entry is an ANSI 256-color index; index 0 is freshly revealed, last is fully cooled.
//   194 = #d7ffd7   157 = #afffaf   120 = #87ff87
//    83 = #5fff5f    46 = #00ff00    40 = #00d700    34 = #00af00
const GRADIENT: &[u8] = &[194, 157, 120, 83, 46, 40, 34];

struct StyledChar {
    ch: char,
    color_prefix: String,
}

/// Parse ANSI-colored text into visible characters with their associated color sequences.
fn parse_styled(input: &str) -> Vec<StyledChar> {
    let mut result = Vec::new();
    let mut current_color = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Start of escape sequence
            let mut seq = String::new();
            seq.push(ch);
            if let Some(&next) = chars.peek() {
                if next == '[' {
                    seq.push(chars.next().unwrap());
                    // Read until we hit a letter (the terminator)
                    while let Some(&c) = chars.peek() {
                        seq.push(chars.next().unwrap());
                        if c.is_ascii_alphabetic() {
                            break;
                        }
                    }
                    // Only track SGR sequences (ending with 'm')
                    if seq.ends_with('m') {
                        if seq == "\x1b[0m" || seq == "\x1b[m" {
                            current_color.clear();
                        } else {
                            current_color.push_str(&seq);
                        }
                    }
                    // Non-SGR CSI sequences are stripped
                }
            }
        } else if !ch.is_control() || ch == '\t' {
            result.push(StyledChar {
                ch,
                color_prefix: current_color.clone(),
            });
        }
    }

    result
}

fn color256(buf: &mut String, idx: u8) {
    write!(buf, "\x1b[38;5;{}m", idx).unwrap();
}

fn cooldown_color(age: usize) -> u8 {
    let steps = GRADIENT.len() - 1;
    let idx = ((age * steps) / COOLDOWN_FRAMES).min(steps);
    GRADIENT[idx]
}

fn render_frame(styled: &[StyledChar], n: usize, frame: usize, buf: &mut String) {
    buf.clear();

    let revealed = if frame >= 2 { (frame - 2).min(n) } else { 0 };
    let space_for_spinner = n.saturating_sub(revealed);
    let has_spinner = frame >= 2 && space_for_spinner > 0;

    // Revealed text: each char cools from HOT→target based on frames since it was revealed.
    for (i, sc) in styled[..revealed].iter().enumerate() {
        let age = frame.saturating_sub(i + 3);
        if age >= COOLDOWN_FRAMES {
            // Fully cooled: snap to the character's real color
            buf.push_str("\x1b[0m");
            buf.push_str(&sc.color_prefix);
        } else {
            color256(buf, cooldown_color(age));
        }
        buf.push(sc.ch);
    }

    // Spinner: bright white
    if has_spinner {
        write!(buf, "\x1b[97m{}", SPINNERS[(frame - 2) % SPINNERS.len()]).unwrap();
    }

    buf.push_str("\x1b[0m");
}

fn main() {
    let stdin = io::stdin();
    let raw_input = if !stdin.is_terminal() {
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
        env::args().skip(1).collect::<Vec<_>>().join(" ")
    };

    if raw_input.is_empty() {
        return;
    }

    let styled = parse_styled(&raw_input);
    let n = styled.len();
    let total_frames = n + COOLDOWN_FRAMES;

    let mut frame_buf = String::with_capacity(n * 16);

    // Write animation frames directly to the terminal (/dev/tty) so they're
    // visible even when stdout is captured (e.g. by fish's fish_prompt).
    // The final prompt goes to stdout for fish to use.
    if let Ok(mut tty) = OpenOptions::new().write(true).open("/dev/tty") {
        write!(tty, "\x1b[?25l").unwrap(); // hide cursor
        for frame in 1..=total_frames {
            render_frame(&styled, n, frame, &mut frame_buf);
            write!(tty, "\r{}", frame_buf).unwrap();
            tty.flush().unwrap();
            thread::sleep(Duration::from_millis(FRAME_DELAY_MS));
        }
        // Clear animation line before final output
        write!(tty, "\r\x1b[K").unwrap();
        write!(tty, "\x1b[?25h").unwrap(); // restore cursor
        tty.flush().unwrap();
    }

    // Final output to stdout (what fish captures as the prompt)
    let stdout = io::stdout();
    let mut out = stdout.lock();
    write!(out, "{}", raw_input).unwrap();
    out.flush().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plain_text() {
        let styled = parse_styled("hello");
        assert_eq!(styled.len(), 5);
        assert_eq!(styled[0].ch, 'h');
        assert!(styled[0].color_prefix.is_empty());
    }

    #[test]
    fn parse_colored_text() {
        let styled = parse_styled("\x1b[36mhello\x1b[0m world");
        assert_eq!(styled.len(), 11); // "hello world"
        assert_eq!(styled[0].ch, 'h');
        assert_eq!(styled[0].color_prefix, "\x1b[36m");
        assert_eq!(styled[5].ch, ' ');
        assert!(styled[5].color_prefix.is_empty()); // after reset
    }

    #[test]
    fn parse_stacked_colors() {
        let styled = parse_styled("\x1b[1m\x1b[36mhi\x1b[0m");
        assert_eq!(styled.len(), 2);
        assert_eq!(styled[0].color_prefix, "\x1b[1m\x1b[36m");
    }

    #[test]
    fn render_frame_first_frame_empty() {
        let styled = parse_styled("abc");
        let mut buf = String::new();
        render_frame(&styled, 3, 1, &mut buf);
        // Frame 1: nothing revealed, spinner not yet started
        assert!(!buf.contains('a'));
    }

    #[test]
    fn render_frame_reveals_chars() {
        let styled = parse_styled("ab");
        let mut buf = String::new();
        render_frame(&styled, 2, 3, &mut buf);
        // Frame 3: 1 char revealed
        assert!(buf.contains('a'));
    }

    #[test]
    fn empty_input_produces_no_output() {
        let styled = parse_styled("");
        assert!(styled.is_empty());
    }

    #[test]
    fn non_sgr_sequences_stripped() {
        // Cursor movement sequence \x1b[H should be stripped
        let styled = parse_styled("\x1b[Hhello");
        assert_eq!(styled.len(), 5);
        assert!(styled[0].color_prefix.is_empty());
    }
}
