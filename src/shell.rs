use std::env;

/// Wraps every ANSI escape sequence in `%{...%}` so zsh counts prompt width correctly.
pub fn wrap_ansi_for_zsh(input: &str) -> String {
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

pub fn is_zsh() -> bool {
    env::var("ZSH_VERSION").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_unchanged() {
        assert_eq!(wrap_ansi_for_zsh("hello world"), "hello world");
    }

    #[test]
    fn sgr_sequence_wrapped() {
        assert_eq!(wrap_ansi_for_zsh("\x1b[36m"), "%{\x1b[36m%}");
    }

    #[test]
    fn reset_sequence_wrapped() {
        assert_eq!(wrap_ansi_for_zsh("\x1b[0m"), "%{\x1b[0m%}");
    }

    #[test]
    fn color_256_sequence_wrapped() {
        // Multi-parameter SGR — used by animation cooldown gradients written to stdout
        assert_eq!(wrap_ansi_for_zsh("\x1b[38;5;196m"), "%{\x1b[38;5;196m%}");
    }

    #[test]
    fn cursor_show_sequence_wrapped() {
        // DEC private mode sequence — appended to zsh stdout as cursor restore
        assert_eq!(wrap_ansi_for_zsh("\x1b[?25h"), "%{\x1b[?25h%}");
    }

    #[test]
    fn realistic_prompt_all_sequences_wrapped() {
        // Representative of what fish pipes to zest: cyan path + bright-cyan arrow
        let input = "\x1b[36m~/projects\x1b[0m \x1b[96m❯ \x1b[0m";
        let expected = "%{\x1b[36m%}~/projects%{\x1b[0m%} %{\x1b[96m%}❯ %{\x1b[0m%}";
        assert_eq!(wrap_ansi_for_zsh(input), expected);
    }

    #[test]
    fn non_csi_escape_passed_through() {
        // Non-CSI escapes (no '[') are not wrappable — pass through as-is
        assert_eq!(wrap_ansi_for_zsh("\x1bM"), "\x1bM");
    }
}
