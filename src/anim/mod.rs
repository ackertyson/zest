mod flames;
mod green_flash;
mod lightning;
mod matrix;
mod scan;

use crate::style::StyledChar;

pub use flames::Flames;
pub use green_flash::GreenFlash;

pub const DEFAULT: &str = "green-flash";
pub const FRAME_DELAY_MS: u64 = 10;

pub const LIST: &[(&str, &str)] = &[
    ("green-flash", "Green cooling gradient sweep"),
    ("flames", "Orange-to-red fire sweep with flickering dot-matrix characters"),
    ("flames-blue", "Blue fire sweep with flickering dot-matrix characters"),
    ("flames-green", "Green fire sweep with flickering dot-matrix characters"),
    ("flames-purple", "Purple fire sweep with flickering dot-matrix characters"),
    ("flames-pink", "Hot pink/magenta fire sweep with flickering dot-matrix characters"),
    ("matrix", "Random ASCII decodes into correct chars, green gradient"),
    ("scan", "CRT phosphor sweep, brief white afterglow"),
    ("lightning", "Instant reveal with bright yellow flash band sweep"),
];

pub trait Animation {
    fn cooldown_frames(&self) -> usize;
    fn total_frames(&self, styled: &[StyledChar]) -> usize {
        styled.len() + self.cooldown_frames()
    }
    fn render_frame(&self, styled: &[StyledChar], frame: usize, buf: &mut String);
}

pub fn cooldown_color(age: usize, cooldown_frames: usize, gradient: &[u8]) -> u8 {
    let steps = gradient.len() - 1;
    let idx = ((age * steps) / cooldown_frames).min(steps);
    gradient[idx]
}

/// Splitmix64-style hash of position + frame — used by animations for deterministic
/// per-cell randomness that avalanches all input bits.
pub(super) fn hash(pos: usize, frame: usize) -> usize {
    let mut h = pos.wrapping_add(frame.wrapping_mul(0x9e3779b97f4a7c15));
    h = (h ^ (h >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94d049bb133111eb);
    h ^ (h >> 31)
}

pub(super) fn revealed(frame: usize, n: usize) -> usize {
    if frame >= 2 { (frame - 2).min(n) } else { 0 }
}

pub(super) fn last_content(styled: &[StyledChar]) -> usize {
    styled.iter().rposition(|sc| !sc.ch.is_whitespace()).unwrap_or(styled.len())
}

pub(super) fn has_leading(frame: usize, revealed: usize, n: usize, last_content: usize) -> bool {
    frame >= 2 && revealed < n && revealed < last_content
}

pub fn resolve(name: &str) -> Option<Box<dyn Animation>> {
    match name {
        "flames"        => Some(Box::new(Flames { gradient: flames::GRADIENT })),
        "flames-blue"   => Some(Box::new(Flames { gradient: flames::GRADIENT_BLUE })),
        "flames-green"  => Some(Box::new(Flames { gradient: flames::GRADIENT_GREEN })),
        "flames-purple" => Some(Box::new(Flames { gradient: flames::GRADIENT_PURPLE })),
        "flames-pink"   => Some(Box::new(Flames { gradient: flames::GRADIENT_PINK })),
        "green-flash"   => Some(Box::new(GreenFlash)),
        "matrix"        => Some(Box::new(matrix::Matrix)),
        "scan"          => Some(Box::new(scan::Scan)),
        "lightning"     => Some(Box::new(lightning::Lightning)),
        _ => None,
    }
}
