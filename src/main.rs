use log::info;

use crate::backend::{OverlayArgs, OverlayBackend, wayland::WaylandBackend, winit::WinitBackend};
use std::env;

mod backend;
mod render;

pub fn pick_backend() -> Box<dyn OverlayBackend> {
    let args = OverlayArgs {
        width: 1024,
        height: 1024,
    };

    let session = env::var("XDG_SESSION_TYPE").ok();

    match session.as_deref() {
        Some("wayland") => {
            info!("Starting with wayland backend");
            return Box::new(WaylandBackend::new(&args).unwrap());
        }
        _ => {
            info!("Starting with winit backend");
            return Box::new(WinitBackend::new(&args));
        }
    }

}

fn main() {
    stderrlog::new()
        .verbosity(log::LevelFilter::Trace)
        .module(module_path!())
        .init()
        .unwrap();

    info!("Starting overlay!");

    let mut overlay = pick_backend();
    overlay.run_event_queue();
}
