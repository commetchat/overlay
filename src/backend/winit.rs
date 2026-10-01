use std::num::NonZeroU32;
use std::sync::Arc;

use log::info;
use piet_tiny_skia::Cache;
use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize, Position};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crate::backend::{OverlayArgs, OverlayBackend};
use crate::render::render_state::RenderState;

#[cfg(all(target_os = "linux"))]
use winit::platform::x11::{WindowAttributesExtX11, WindowType};

#[derive(Default)]
pub struct WinitBackend {
    window: Option<Arc<Window>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    contaxt: Option<Context<Arc<Window>>>,
    size: PhysicalSize<u32>,
}

impl WinitBackend {
    pub fn new(args: &OverlayArgs) -> WinitBackend {
        info!("new size: {} x {}", args.width, args.height);

        let result = WinitBackend {
            window: None,
            surface: None,
            contaxt: None,
            size: PhysicalSize {
                width: args.width,
                height: args.height,
            },
        };

        info!(
            "winit: created window with size: {} x {}",
            result.size.width, result.size.height
        );

        return result;
    }
}

impl ApplicationHandler for WinitBackend {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        info!(
            "winit: creating window with size: {} x {}",
            self.size.width, self.size.height
        );

        let mut position = Position::Physical(PhysicalPosition { x: 0, y: 0 });

        if let Some(monitor) = event_loop.primary_monitor() {
            let monitor_pos = monitor.position();
            position = Position::Physical(monitor_pos);
        }

        let mut attributes = Window::default_attributes()
            .with_inner_size(self.size)
            .with_transparent(true)
            .with_position(position)
            .with_decorations(false);

        #[cfg(all(target_os = "linux"))]
        {
            attributes = attributes.with_x11_window_type(vec![WindowType::Utility]);
        }

        let window = event_loop.create_window(attributes).unwrap();

        window.set_cursor_hittest(false).unwrap();
        window.set_window_level(winit::window::WindowLevel::AlwaysOnTop);

        let window = Arc::new(window);

        let context = softbuffer::Context::new(window.clone()).unwrap();

        let mut surface = Surface::new(&context, window.clone()).unwrap();

        surface
            .resize(
                NonZeroU32::new(self.size.width).unwrap(),
                NonZeroU32::new(self.size.height).unwrap(),
            )
            .unwrap();

        self.contaxt = Some(context);
        self.surface = Some(surface);
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.draw();
            }
            WindowEvent::Resized(size) => {
                self.size = size;

                if size.width != 0 && size.height != 0 {
                    if let Some(surface) = &mut self.surface {
                        surface
                            .resize(
                                NonZeroU32::new(size.width).unwrap(),
                                NonZeroU32::new(size.height).unwrap(),
                            )
                            .unwrap();
                    }
                }
            }
            _ => (),
        }
    }
}

impl OverlayBackend for WinitBackend {
    fn show(&self) {}

    fn run_event_queue(&mut self) {
        let event_loop = EventLoop::new().unwrap();

        event_loop.set_control_flow(ControlFlow::Poll);

        event_loop.run_app(self).unwrap();
    }

    fn draw(&mut self) {
        let Some(window) = &self.window else {
            return;
        };

        let width = self.size.width as usize;
        let height = self.size.height as usize;

        if width == 0 || height == 0 {
            return;
        }

        let mut pixels = vec![0u8; width * height * 4];

        let state = &RenderState::new();

        let mut cache = Cache::new();

        let mut pixmap = tiny_skia::PixmapMut::from_bytes(&mut pixels, width as u32, height as u32)
            .ok_or("invalid pixmap")
            .unwrap();

        pixmap.fill(tiny_skia::Color::TRANSPARENT);
        let mut ctx = cache.render_context(pixmap);
        state.draw(&mut ctx);

        let surface = self.surface.as_mut().unwrap();

        let mut buffer = surface.buffer_mut().unwrap();

        // Convert rgba to bgra
        for i in 0..width * height {
            let r = pixels[i * 4] as u32;
            let g = pixels[i * 4 + 1] as u32;
            let b = pixels[i * 4 + 2] as u32;
            let a = pixels[i * 4 + 3] as u32;
            buffer[i] = (a << 24) | (r << 16) | (g << 8) | b;
        }

        buffer.present().unwrap();

        window.request_redraw();
    }
}
