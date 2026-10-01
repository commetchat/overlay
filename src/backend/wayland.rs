use std::num::NonZeroU32;

use log::{info, trace};

use piet_tiny_skia::Cache;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_registry,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::{
        WaylandSurface,
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
    },
    shm::{
        Shm, ShmHandler,
        slot::{Buffer, SlotPool},
    },
};

use wayland_client::{
    Connection, EventQueue, QueueHandle,
    globals::registry_queue_init,
    protocol::{wl_output, wl_region::WlRegion, wl_shm, wl_surface},
};

use crate::{
    backend::{OverlayArgs, OverlayBackend},
    render::render_state::RenderState,
};

pub struct WaylandState {
    registry_state: RegistryState,
    output_state: OutputState,

    shm: Shm,

    layer: LayerSurface,

    pool: SlotPool,
    buffer: Option<Buffer>,

    width: u32,
    height: u32,

    cache: Cache,

    configured: bool,
}

pub struct WaylandBackend {
    data: WaylandState,
    event_queue: EventQueue<WaylandState>,
}

impl WaylandBackend {
    pub fn new(args: &OverlayArgs) -> Result<WaylandBackend, Box<dyn std::error::Error>> {
        let connection = Connection::connect_to_env()?;

        let (globals, event_queue) = registry_queue_init(&connection)?;

        let qh: QueueHandle<WaylandState> = event_queue.handle();

        let compositor = CompositorState::bind(&globals, &qh)?;

        let output_state = OutputState::new(&globals, &qh);

        let layer_shell = LayerShell::bind(&globals, &qh)?;

        let shm = Shm::bind(&globals, &qh)?;

        let surface = compositor.create_surface(&qh);

        let layer = layer_shell.create_layer_surface(
            &qh,
            surface,
            Layer::Overlay,
            Some("rust-overlay"),
            None,
        );

        let wl_compositor = compositor.wl_compositor();

        let input_region = wl_compositor.create_region(&qh, ());

        layer.wl_surface().set_input_region(Some(&input_region));
        layer.set_anchor(Anchor::TOP | Anchor::LEFT);

        layer.set_size(args.width, args.height);

        layer.set_keyboard_interactivity(KeyboardInteractivity::None);

        layer.set_exclusive_zone(-1);

        layer.commit();

        let pool_size = (args.width * args.height * 4) as usize;

        let pool = SlotPool::new(pool_size, &shm)?;

        return Ok(WaylandBackend {
            data: WaylandState {
                registry_state: RegistryState::new(&globals),
                output_state: output_state,
                shm,
                layer,
                pool,
                buffer: None,
                cache: Cache::new(),
                width: args.width,
                height: args.height,
                configured: false,
            },
            event_queue,
        });
    }
}

impl WaylandState {
    fn draw(&mut self, state: &RenderState) {
        use std::time::Instant;
        let now = Instant::now();

        let width = self.width;
        let height = self.height;
        let stride = width * 4;

        let buffer = self.buffer.get_or_insert_with(|| {
            self.pool
                .create_buffer(
                    width as i32,
                    height as i32,
                    stride as i32,
                    wl_shm::Format::Argb8888,
                )
                .expect("failed to create wl_shm buffer")
                .0
        });

        let canvas = match self.pool.canvas(buffer) {
            Some(canvas) => canvas,
            None => {
                let (new_buffer, canvas) = self
                    .pool
                    .create_buffer(
                        width as i32,
                        height as i32,
                        stride as i32,
                        wl_shm::Format::Argb8888,
                    )
                    .unwrap();

                *buffer = new_buffer;
                canvas
            }
        };

        let mut pixmap = tiny_skia::PixmapMut::from_bytes(canvas, width, height)
            .ok_or("invalid pixmap")
            .unwrap();

        pixmap.fill(tiny_skia::Color::TRANSPARENT);
        let mut ctx = self.cache.render_context(pixmap);
        state.draw(&mut ctx);

        // swap from rgba to bgra
        for px in canvas.chunks_exact_mut(4) {
            px.swap(0, 2);
        }

        let elapsed = now.elapsed();
        trace!("Frame rendered in: {:.2?}", elapsed);

        self.layer
            .wl_surface()
            .damage_buffer(0, 0, width as i32, height as i32);

        buffer.attach_to(self.layer.wl_surface()).unwrap();

        self.layer.commit();
    }
}

impl OverlayBackend for WaylandBackend {
    fn show(&self) {}

    fn run_event_queue(&mut self) {
        let mut iter = 0;
        loop {
            self.event_queue.dispatch_pending(&mut self.data).unwrap();

            iter += 1;
            if self.data.configured {
                self.data.draw(&RenderState { iteration: iter });
            }

            self.event_queue.blocking_dispatch(&mut self.data).unwrap();
            info!("Running loop!")
        }
    }

    fn draw(&mut self) {
        self.data.draw(&RenderState::new());
    }
}

impl CompositorHandler for WaylandState {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
        trace!("wayland: scale_factor_changed");
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
        trace!("wayland: transform_changed");
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        trace!("wayland: frame");
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        trace!("wayland: surface_enter");
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        trace!("wayland: surface_leave");
    }
}

impl wayland_client::Dispatch<WlRegion, ()> for WaylandState {
    fn event(
        _state: &mut WaylandState,
        _proxy: &WlRegion,
        _event: <WlRegion as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<WaylandState>,
    ) {
    }
}

impl OutputHandler for WaylandState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for WaylandState {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        todo!()
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        self.width = NonZeroU32::new(configure.new_size.0).map_or(self.width, NonZeroU32::get);

        self.height = NonZeroU32::new(configure.new_size.1).map_or(self.height, NonZeroU32::get);

        info!("overlay configured: {}x{}", self.width, self.height);

        if !self.configured {
            self.configured = true;
            self.draw(&RenderState::new());
        }
    }
}

impl ShmHandler for WaylandState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

delegate_registry!(WaylandState);

impl ProvidesRegistryState for WaylandState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![OutputState];
}

smithay_client_toolkit::delegate_dispatch2!(WaylandState);
