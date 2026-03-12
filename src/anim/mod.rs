mod flames;
mod green_flash;

use crate::style::StyledChar;

pub use flames::Flames;
pub use green_flash::GreenFlash;

pub const DEFAULT: &str = "green-flash";

pub const LIST: &[(&str, &str)] = &[
    ("green-flash", "Green cooling gradient sweep"),
    ("flames", "Orange-to-red fire sweep with flickering dot-matrix characters"),
    ("flames-blue", "Blue fire sweep with flickering dot-matrix characters"),
    ("flames-green", "Green fire sweep with flickering dot-matrix characters"),
    ("flames-purple", "Purple fire sweep with flickering dot-matrix characters"),
];

pub trait Animation {
    fn total_frames(&self, styled: &[StyledChar]) -> usize;
    fn frame_delay_ms(&self) -> u64;
    fn render_frame(&self, styled: &[StyledChar], frame: usize, buf: &mut String);
}

pub fn cooldown_color(age: usize, cooldown_frames: usize, gradient: &[u8]) -> u8 {
    let steps = gradient.len() - 1;
    let idx = ((age * steps) / cooldown_frames).min(steps);
    gradient[idx]
}

pub fn resolve(name: &str) -> Option<Box<dyn Animation>> {
    match name {
        "flames"        => Some(Box::new(Flames { gradient: flames::GRADIENT })),
        "flames-blue"   => Some(Box::new(Flames { gradient: flames::GRADIENT_BLUE })),
        "flames-green"  => Some(Box::new(Flames { gradient: flames::GRADIENT_GREEN })),
        "flames-purple" => Some(Box::new(Flames { gradient: flames::GRADIENT_PURPLE })),
        "green-flash"   => Some(Box::new(GreenFlash)),
        _ => None,
    }
}
