use std::collections::BTreeMap;

use piet::{
    Color, FontFamily, ImageBuf, RenderContext, Text, TextLayout, TextLayoutBuilder, UnitPoint,
    kurbo::{Circle, Point, Rect},
};

use tiny_skia::PixmapMut;

use crate::VoipMemberState;

pub enum ImageState {
    Pending,
    Received(ImageBuf),
    FailedToDecode,
}

pub struct RenderState {
    pub iteration: i32,
    pub users: BTreeMap<String, VoipMemberState>,
    pub images: BTreeMap<String, ImageState>,
    pub item_height: u32,
    pub changed: bool,
    pub mute_icon: Option<ImageBuf>,
}

impl RenderState {
    pub fn new() -> RenderState {
        return RenderState {
            iteration: 0,
            changed: true,
            item_height: 32,
            images: BTreeMap::new(),
            users: BTreeMap::new(),
            mute_icon: None,
        };
    }

    pub fn draw(&self, ctx: &mut piet_tiny_skia::RenderContext<'_, PixmapMut<'_>>) {
        // if self.users.is_empty() {
        //     return;
        // }

        let text_size = self.item_height as f64;

        let mut y_position = text_size * 3.0;
        let x_position = text_size * 2.0;

        let font = ctx
            .text()
            .font_family("Roboto")
            .unwrap_or(FontFamily::MONOSPACE);

        let draw_background = false;
        if draw_background {
            let height = (y_position * 2.0) + (text_size * 2.0 * (self.users.len() as f64));

            let gradient = piet::LinearGradient::new(
                UnitPoint::new(0.0, 0.0),
                UnitPoint::new(0.5, 0.5),
                (
                    piet::Color::rgba8(0, 0, 0, 100),
                    piet::Color::rgba8(0, 0, 0, 100),
                    piet::Color::TRANSPARENT,
                ),
            );

            ctx.fill(Rect::new(0.0, 0.0, text_size * 12.5, height), &gradient);
        }

        for (_user_id, entry) in self.users.iter() {
            let text = entry.display_name.clone();

            let mut alpha = 255;

            if entry.muted {
                alpha = 120;
            }

            let draw_image_outline = entry.talking;

            let show_mute_icon = entry.muted;

            let shadow = ctx
                .text()
                .new_text_layout(text.clone())
                .font(font.clone(), text_size)
                .text_color(Color::rgba8(0, 0, 0, alpha))
                .build()
                .unwrap();

            let highlight = ctx
                .text()
                .new_text_layout(text.clone())
                .font(font.clone(), text_size)
                .text_color(Color::rgba8(255, 255, 255, alpha))
                .build()
                .unwrap();

            let mut x = x_position;

            let circle_radius = text_size * 0.75;
            let y_pad = text_size * 0.2;
            let circle_position = Point::new(
                x + circle_radius,
                y_position + (circle_radius / 2.0) + y_pad,
            );

            if draw_image_outline {
                ctx.fill(
                    Circle::new(circle_position, circle_radius + text_size * 0.2),
                    &Color::rgba8(120, 120, 255, 255),
                );
            }

            let img = match &entry.avatar_url {
                Some(img) => match self.images.get(img) {
                    Some(state) => match state {
                        ImageState::Pending => None,
                        ImageState::Received(image_buf) => Some(image_buf),
                        ImageState::FailedToDecode => None,
                    },
                    None => None,
                },
                None => None,
            };

            if let Some(img) = img {
                let img = &img.to_image(ctx);

                _ = ctx.with_save(|rc| {
                    rc.clip(Circle::new(circle_position, circle_radius));

                    let rect = Rect {
                        x0: circle_position.x - circle_radius,
                        x1: circle_position.x + circle_radius,
                        y0: circle_position.y - circle_radius,
                        y1: circle_position.y + circle_radius,
                    };

                    rc.draw_image(img, rect, piet::InterpolationMode::Bilinear);

                    Ok(())
                });
            } else {
                ctx.fill(
                    Circle::new(circle_position, circle_radius),
                    &Color::rgba8(45, 45, 45, alpha),
                );
            }

            x += circle_radius * 2.0;
            x += text_size * 0.5;

            if show_mute_icon {
                let icon_position = Point::new(
                    x + circle_radius + shadow.size().width + (y_pad * 4.0),
                    y_position + (circle_radius / 2.0) + (y_pad * 1.5),
                );

                let radius = circle_radius * 0.8;

                let rect = Rect {
                    x0: icon_position.x - radius,
                    x1: icon_position.x + radius,
                    y0: icon_position.y - radius,
                    y1: icon_position.y + radius,
                };

                let img =  self.mute_icon.as_ref().unwrap().to_image(ctx);

                ctx.draw_image(&img, rect, piet::InterpolationMode::Bilinear);

            }

            ctx.draw_text(&shadow, Point::new(x, y_position));
            ctx.draw_text(&highlight, Point::new(x - 1.0, y_position - 1.0));

            y_position += text_size * 2.0;
        }
    }
}
