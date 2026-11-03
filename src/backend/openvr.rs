use core::panic;
use std::{thread, time::Duration};

use log::info;
use openvr::overlay::OverlayHandle;
use piet_tiny_skia::Cache;

use crate::{
    backend::{OverlayArgs, OverlayBackend},
    render::render_state::RenderState,
};

pub struct OpenVRBackend {
    buffer: Vec<u8>,
    width: usize,
    height: usize,
    context: openvr::Context,
    overlay: openvr::Overlay,
    system: openvr::System,
    handle: OverlayHandle,
    cache: piet_tiny_skia::Cache,
}

fn make_transform(
    translation: [f32; 3],
    rotation_radians: [f32; 3],
    scale: [f32; 3],
) -> openvr::pose::Matrix3x4 {
    let (sx, cx) = rotation_radians[0].sin_cos();
    let (sy, cy) = rotation_radians[1].sin_cos();
    let (sz, cz) = rotation_radians[2].sin_cos();

    let r00 = cz * cy;
    let r01 = cz * sy * sx - sz * cx;
    let r02 = cz * sy * cx + sz * sx;

    let r10 = sz * cy;
    let r11 = sz * sy * sx + cz * cx;
    let r12 = sz * sy * cx - cz * sx;

    let r20 = -sy;
    let r21 = cy * sx;
    let r22 = cy * cx;

    openvr::pose::Matrix3x4 {
        0: [
            [
                r00 * scale[0],
                r01 * scale[1],
                r02 * scale[2],
                translation[0],
            ],
            [
                r10 * scale[0],
                r11 * scale[1],
                r12 * scale[2],
                translation[1],
            ],
            [
                r20 * scale[0],
                r21 * scale[1],
                r22 * scale[2],
                translation[2],
            ],
        ],
    }
}

impl OpenVRBackend {
    fn get_context() -> openvr::Context {
        for i in 0..10 {
            info!("Attempting to launch openvr");

            unsafe {
                match openvr::init(openvr::ApplicationType::Overlay) {
                    Ok(context) => {
                        info!("Got openvr context");
                        return context;
                    }
                    Err(_) => {
                        info!("Failed (Attempt {})", i);
                        thread::sleep(Duration::from_secs(3));
                    }
                }
            }
        }

        panic!()
    }

    pub fn new(args: &OverlayArgs) -> OpenVRBackend {
        let context = Self::get_context();

        let system = context.system().unwrap();

        let mut overlay = context.overlay().unwrap();

        let handle = overlay
            .create_overlay("chat.commet\x00", "Commet Overlay\x00")
            .expect("Failed to create overlay");

        let controller_index = openvr::TrackedDeviceIndex(0);

        let transform = make_transform([-0.15, 0.05, -1.0], [0.3, 0.3, 0.0], [1.0, 1.0, 1.0]);

        overlay
            .set_transform_tracked_device_relative(handle, controller_index, &transform)
            .unwrap();

        let width = args.width as usize;
        let height = args.height as usize;

        let buffer = vec![0u8; width * height * 4];

        overlay
            .set_width(handle, 0.5)
            .expect("Failed to set overlay width");

        OpenVRBackend {
            context,
            overlay,
            handle,
            height,
            width,
            buffer,
            system,
            cache: Cache::new(),
        }
    }

    fn draw_overlay(&mut self, state: &RenderState) {
        let mut pixmap = tiny_skia::PixmapMut::from_bytes(
            &mut self.buffer,
            self.width as u32,
            self.height as u32,
        )
        .ok_or("invalid pixmap")
        .unwrap();

        pixmap.fill(tiny_skia::Color::TRANSPARENT);

        let mut ctx = self.cache.render_context(pixmap);

        state.draw(&mut ctx);

        self.overlay
            .set_raw_data(self.handle, &self.buffer, self.width, self.height, 4)
            .expect("Failed to upload overlay pixels");
    }
}

impl OverlayBackend for OpenVRBackend {
    fn show(&mut self) {
        self.overlay.set_visibility(self.handle, true).unwrap();
    }

    fn run_event_queue(&mut self) {}

    fn poll_event_queue(&mut self, state: &crate::render::render_state::RenderState) {
        while let Some(event) = self.system.poll_next_event() {
            info!("Received event: {:?}", event.event);

            match event.event {
                openvr::system::Event::Quit(_) => {
                    info!("Acknowledging quit event");

                    self.system.acknowledge_quit_exiting();
                }
                _ => (),
            }
        }

        if state.changed {
            self.draw_overlay(state);

            self.show();
        }
    }

    fn draw(&mut self) {}

    fn make_render_state(&mut self) -> RenderState {
        let mut state = RenderState::new(&mut self.cache);
        state.item_height = 32;

        state
    }

    fn dispose(&mut self) {}
}
