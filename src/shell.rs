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
