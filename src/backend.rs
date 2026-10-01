pub mod winit;

#[cfg(target_os = "linux")]
pub mod wayland;

pub struct OverlayArgs {
    pub width: u32,
    pub height: u32,
}

pub trait OverlayBackend {
    fn show(&self);

    fn run_event_queue(&mut self);

    fn draw(&mut self);
}
