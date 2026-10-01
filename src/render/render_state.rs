use piet::{
    Color, FontFamily, RenderContext, Text, TextLayout, TextLayoutBuilder,
    kurbo::{Point, Rect},
};

use tiny_skia::PixmapMut;
pub struct RenderState {
    pub iteration: i32,
}

impl RenderState {
    pub fn new() -> RenderState {
        return RenderState { iteration: 0 };
    }

    pub fn draw(&self, ctx: &mut piet_tiny_skia::RenderContext<'_, PixmapMut<'_>>) {
        let col = Color::hlca(((self.iteration % 360) as f64) * 5.0, 50.0, 50.0, 0.8);
        ctx.fill(Rect::new(0.0, 0.0, 200.0, 32.0), &col);

        let text_size = 12.0;

        let layout = ctx
            .text()
            .new_text_layout("@test:example.org")
            .text_color(Color::rgba8(0, 0, 0, 255))
            .font(FontFamily::MONOSPACE, text_size)
            .build()
            .unwrap();

        let size = layout.image_bounds();

        ctx.fill(
            Rect::new(6.0, 6.0, 8. + size.width(), 8. + size.height()),
            &Color::rgba8(0, 0, 0, 120),
        );

        ctx.draw_text(&layout, Point::new(8., 8.));

        let layout = ctx
            .text()
            .new_text_layout("@test:example.org")
            .text_color(Color::rgba8(255, 255, 255, 255))
            .font(FontFamily::MONOSPACE, text_size)
            .build()
            .unwrap();

        ctx.draw_text(&layout, Point::new(6., 6.));

        ctx.finish().unwrap();
    }
}
