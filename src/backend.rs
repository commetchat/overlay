pub mod wayland;
pub mod winit;

pub struct OverlayArgs {
    pub width: u32,
    pub height: u32,
}

pub trait OverlayBackend {
    fn show(&self);

    fn run_event_queue(&mut self);

    fn draw(&mut self);
}
