mod flames;
mod green_flash;

use crate::style::StyledChar;

pub use flames::Flames;
pub use green_flash::GreenFlash;

pub const DEFAULT: &str = "green-flash";

pub trait Animation {
    fn total_frames(&self, n: usize) -> usize;
    fn frame_delay_ms(&self) -> u64;
    fn render_frame(&self, styled: &[StyledChar], n: usize, frame: usize, buf: &mut String);
}

pub fn resolve(name: &str) -> Option<Box<dyn Animation>> {
    match name {
        "flames" => Some(Box::new(Flames)),
        "green-flash" => Some(Box::new(GreenFlash)),
        _ => None,
    }
}
