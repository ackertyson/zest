use std::fmt::Write;
use std::rc::Rc;

pub struct StyledChar {
    pub ch: char,
    pub color_prefix: Rc<str>,
}

/// Parse ANSI-colored text into visible characters with their associated color sequences.
pub fn parse_styled(input: &str) -> Vec<StyledChar> {
    let mut result = Vec::with_capacity(input.len());
    let mut current_color = String::new();
    let mut shared: Rc<str> = Rc::from("");
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Start of escape sequence
            let mut seq = String::new();
            seq.push(ch);
            if let Some(&next) = chars.peek()
                && next == '['
            {
                seq.push(chars.next().unwrap());
                // Read until we hit a letter (the terminator)
                while let Some(&c) = chars.peek() {
                    seq.push(chars.next().unwrap());
                    if ('\x40'..='\x7e').contains(&c) {
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
                    shared = Rc::from(current_color.as_str());
                }
                // Non-SGR CSI sequences are stripped
            }
        } else if !ch.is_control() || ch == '\t' {
            result.push(StyledChar {
                ch,
                color_prefix: Rc::clone(&shared),
            });
        }
    }

    result
}

pub fn color256(buf: &mut String, idx: u8) {
    write!(buf, "\x1b[38;5;{}m", idx).unwrap();
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
        assert_eq!(&*styled[0].color_prefix, "\x1b[36m");
        assert_eq!(styled[5].ch, ' ');
        assert!(styled[5].color_prefix.is_empty()); // after reset
    }

    #[test]
    fn parse_stacked_colors() {
        let styled = parse_styled("\x1b[1m\x1b[36mhi\x1b[0m");
        assert_eq!(styled.len(), 2);
        assert_eq!(&*styled[0].color_prefix, "\x1b[1m\x1b[36m");
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

    #[test]
    fn non_alphabetic_csi_terminator() {
        // \x1b[2~ is a vt key sequence (Insert key) — '~' is a valid CSI terminator
        let styled = parse_styled("\x1b[2~hello");
        assert_eq!(styled.len(), 5);
        assert_eq!(styled[0].ch, 'h');
        assert!(styled[0].color_prefix.is_empty());
    }

    #[test]
    fn consecutive_chars_share_rc() {
        let styled = parse_styled("\x1b[36mhello");
        assert_eq!(styled.len(), 5);
        // All 5 chars should share the same Rc allocation
        assert!(Rc::ptr_eq(&styled[0].color_prefix, &styled[1].color_prefix));
        assert!(Rc::ptr_eq(&styled[0].color_prefix, &styled[4].color_prefix));
    }
}
