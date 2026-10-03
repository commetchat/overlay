use std::vec;

use piet::{
    Color, FontFamily, RenderContext, Text, TextLayout, TextLayoutBuilder, UnitPoint, kurbo::{Circle, Point, Rect},
};

use tiny_skia::{ PixmapMut};
pub struct RenderState {
    pub iteration: i32,
    pub users: Vec<UserData>,
}

pub enum UserState {
    Idle,
    Talking,
    Muted,
}

pub struct UserData {
    pub display_name: String,
    pub state: UserState,
}

impl RenderState {
    pub fn new() -> RenderState {
        return RenderState {
            iteration: 0,
            users: vec![
                UserData {
                    display_name: "airyz".to_string(),
                    state: UserState::Idle,
                },
                UserData {
                    display_name: "pluto".to_string(),
                    state: UserState::Talking,
                },
                UserData {
                    display_name: "luna".to_string(),
                    state: UserState::Muted,
                },
            ],
        };
    }

    pub fn draw(&self, ctx: &mut piet_tiny_skia::RenderContext<'_, PixmapMut<'_>>) {
        let text_size = 12.0;

        let mut y_position = text_size;
        let x_position = text_size;

        let font = ctx
            .text()
            .font_family("Roboto")
            .unwrap_or(FontFamily::MONOSPACE);

        let height = y_position + (text_size * 2.0 * (self.users.len() as f64));

        let col = Color::hlca(((self.iteration % 360) as f64) * 5.0, 50.0, 50.0, 0.8);
        
        let gradient = piet::LinearGradient::new(UnitPoint::new(0.0, 0.0), UnitPoint::new(0.5, 0.5),  (piet::Color::rgba8(0, 0, 0, 100), piet::Color::rgba8(0, 0, 0, 100), piet::Color::TRANSPARENT));

        
        ctx.fill(
            Rect::new(0.0, 0.0, 200.0, height),
            &gradient,
        );

        for entry in self.users.iter() {
            let text = entry.display_name.clone();

            let alpha = match entry.state {
                UserState::Idle => 255,
                UserState::Talking => 255,
                UserState::Muted => 120,
            };

            let draw_image_outline = match entry.state {
                UserState::Idle => false,
                UserState::Talking => true,
                UserState::Muted => false,
            };

            let show_icon = match entry.state {
                UserState::Idle => false,
                UserState::Talking => false,
                UserState::Muted => true,
            };

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

            ctx.fill(
                Circle::new(circle_position, circle_radius),
                &Color::rgba8(45, 45, 45, alpha),
            );

            x += circle_radius * 2.0;
            x += text_size * 0.5;

            if show_icon {
                let icon_position = Point::new(
                    x + circle_radius + shadow.size().width + y_pad,
                    y_position + (circle_radius / 2.0) + y_pad,
                );

                ctx.fill(
                    Circle::new(icon_position, circle_radius * 0.5),
                    &col
                );
            }

            // let size = shadow.image_bounds();

            // ctx.fill(
            //     Rect::new(
            //         x_position,
            //         y_position,
            //         x_position + size.width() + 6.0,
            //         y_position + size.height() + 2.0,
            //     ),
            //     &Color::rgba8(0, 0, 0, 120),
            // );

            ctx.draw_text(&shadow, Point::new(x, y_position));
            ctx.draw_text(&highlight, Point::new(x - 1.0, y_position - 1.0));

            y_position += text_size * 2.0;
        }
    }
}
