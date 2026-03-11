use std::env;
use std::fmt::Write as FmtWrite;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

const SPINNERS: [char; 4] = ['-', '\\', '|', '/'];
const END_CHARS: usize = 2;
const FRAME_DELAY_MS: u64 = 16;
const COOLDOWN_FRAMES: usize = 12;

// 256-color gradient from hot (bright greenish-white) to resting dark green.
// Each entry is an ANSI 256-color index; index 0 is freshly revealed, last is fully cooled.
//   194 = #d7ffd7   157 = #afffaf   120 = #87ff87
//    83 = #5fff5f    46 = #00ff00    40 = #00d700    34 = #00af00
const GRADIENT: &[u8] = &[194, 157, 120, 83, 46, 40, 34];

const REST_COLOR: u8 = 34;   // #00af00  resting dark green
const PROMPT_COLOR: u8 = 40; // #00d700  slightly brighter for '>'

fn color256(buf: &mut String, idx: u8) {
    write!(buf, "\x1b[38;5;{}m", idx).unwrap();
}

fn cooldown_color(age: usize) -> u8 {
    let steps = GRADIENT.len() - 1;
    let idx = ((age * steps) / COOLDOWN_FRAMES).min(steps);
    GRADIENT[idx]
}

fn render_frame(text: &[char], n: usize, frame: usize, buf: &mut String) {
    buf.clear();

    let end_len = END_CHARS.min(frame).min(n);
    let max_revealed = n.saturating_sub(end_len);
    let revealed = if frame >= 2 { (frame - 2).min(max_revealed) } else { 0 };
    let space_for_spinner = max_revealed.saturating_sub(revealed);
    let has_spinner = frame >= 2 && space_for_spinner > 0;

    // Revealed text: each char cools from HOT→REST based on frames since it was revealed.
    // Char i is first revealed at frame (i + 3).
    for (i, &ch) in text[..revealed].iter().enumerate() {
        let age = frame.saturating_sub(i + 3);
        color256(buf, cooldown_color(age));
        buf.push(ch);
    }

    // Spinner: bright white (standard 16-color, works everywhere)
    if has_spinner {
        write!(buf, "\x1b[97m{}", SPINNERS[(frame - 2) % SPINNERS.len()]).unwrap();
    }

    // End section: space in REST, '>' in PROMPT
    if end_len == 2 {
        color256(buf, REST_COLOR);
        buf.push(text[n - 2]);
    }
    color256(buf, PROMPT_COLOR);
    buf.push(text[n - 1]);

    buf.push_str("\x1b[0m");
}

fn main() {
    let text: String = env::args().skip(1).collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        eprintln!("Usage: zest <text>");
        std::process::exit(1);
    }

    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let total_frames = n + COOLDOWN_FRAMES;

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut frame_buf = String::with_capacity(n * 16);

    write!(out, "\x1b[?25l").unwrap(); // hide cursor
    for frame in 1..=total_frames {
        render_frame(&chars, n, frame, &mut frame_buf);
        write!(out, "\r{}", frame_buf).unwrap();
        out.flush().unwrap();
        thread::sleep(Duration::from_millis(FRAME_DELAY_MS));
    }
    write!(out, "\x1b[?25h").unwrap(); // restore cursor
    writeln!(out).unwrap();
}
