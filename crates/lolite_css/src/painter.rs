use crate::{
    engine::RenderNode,
    style::{Length, Rgba},
};
use skia_safe::{Canvas, Color, Color4f, Paint, RRect, Rect};

pub struct Painter<'a> {
    canvas: &'a Canvas,
}

impl<'a> Painter<'a> {
    pub fn new(canvas: &'a Canvas) -> Self {
        Self { canvas }
    }

    pub fn paint(&mut self, root: &RenderNode) {
        self.canvas.clear(Color::WHITE);
        self.paint_node(root);
    }

    fn paint_node(&mut self, node: &RenderNode) {
        // Draw the node's background color if it has one
        let style = &node.style;

        let client_rect = Rect::new(
            node.bounds.x as f32,
            node.bounds.y as f32,
            (node.bounds.x + node.bounds.width) as f32,
            (node.bounds.y + node.bounds.height) as f32,
        );

        let client_rrect = if let Some(border_radius) = &style.border_radius {
            RRect::new_rect_xy(
                client_rect,
                border_radius.top_left.to_px() as f32,
                border_radius.top_right.to_px() as f32,
            )
        } else {
            RRect::new_rect_xy(client_rect, 0.0, 0.0)
        };

        if let Some(background_color) = &style.background_color {
            let paint = Paint::new(background_color.to_color4f(), None);

            self.canvas.draw_rrect(client_rrect, &paint);
        }

        if let Some(border_width) = &style.border_width {
            let color = style.border_color.as_ref().unwrap_or(&Rgba {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            });

            let mut paint = Paint::new(color.to_color4f(), None);
            paint.set_style(skia_safe::paint::Style::Stroke);
            paint.set_stroke_width(border_width.to_px() as f32);
            paint.set_anti_alias(true);
            self.canvas.draw_rrect(client_rrect, &paint);
        }

        // Draw the node's text if it has any
        if let Some(text) = &node.text {
            let mut paint = Paint::default();
            paint.set_color(Color::BLACK);

            let x = node.bounds.x as f32;
            let y = (node.bounds.y + node.bounds.height / 2.0) as f32;

            let typeface = skia_safe::FontMgr::default()
                .match_family("Arial")
                .match_style(skia_safe::FontStyle::normal())
                .expect("Failed to load font");

            let font = skia_safe::Font::new(typeface, 12.0);

            self.canvas.draw_str(text, (x, y), &font, &paint);
        }
        // Recursively paint the children
        for child in &node.children {
            self.paint_node(child);
        }
    }
}

// Helper method to convert Length to pixels
#[allow(unused)]
trait ToPx {
    fn to_px(&self) -> f64;
}

impl ToPx for Length {
    fn to_px(&self) -> f64 {
        match self {
            Length::Px(value) => *value,
            _ => 0.0, // Handle other cases as needed
        }
    }
}

pub(crate) trait ToColor4f {
    fn to_color4f(&self) -> Color4f;
}

impl ToColor4f for Rgba {
    fn to_color4f(&self) -> Color4f {
        let color = Color::from_argb(self.a, self.r, self.g, self.b);
        Color4f::new(
            color.r() as f32 / 255.0,
            color.g() as f32 / 255.0,
            color.b() as f32 / 255.0,
            color.a() as f32 / 255.0,
        )
    }
}
