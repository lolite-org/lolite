use crate::{
    layout::RenderNode,
    style::{BorderStyle, Length, Rgba},
    text::{FontSpec, SkiaTextMeasurer},
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

        let client_rrect = if style.border_radius.is_empty() {
            RRect::new_rect_xy(client_rect, 0.0, 0.0)
        } else {
            let tl = style
                .border_radius
                .top_left
                .as_ref()
                .map(|r| (r.x.to_px() as f32, r.y.to_px() as f32));
            let tr = style
                .border_radius
                .top_right
                .as_ref()
                .map(|r| (r.x.to_px() as f32, r.y.to_px() as f32));
            let br = style
                .border_radius
                .bottom_right
                .as_ref()
                .map(|r| (r.x.to_px() as f32, r.y.to_px() as f32));
            let bl = style
                .border_radius
                .bottom_left
                .as_ref()
                .map(|r| (r.x.to_px() as f32, r.y.to_px() as f32));

            RRect::new_rect_radii(
                client_rect,
                &[
                    skia_safe::Vector::new(
                        tl.map(|v| v.0).unwrap_or(0.0),
                        tl.map(|v| v.1).unwrap_or(0.0),
                    ),
                    skia_safe::Vector::new(
                        tr.map(|v| v.0).unwrap_or(0.0),
                        tr.map(|v| v.1).unwrap_or(0.0),
                    ),
                    skia_safe::Vector::new(
                        br.map(|v| v.0).unwrap_or(0.0),
                        br.map(|v| v.1).unwrap_or(0.0),
                    ),
                    skia_safe::Vector::new(
                        bl.map(|v| v.0).unwrap_or(0.0),
                        bl.map(|v| v.1).unwrap_or(0.0),
                    ),
                ],
            )
        };

        if let Some(background_color) = &style.background_color {
            let paint = Paint::new(background_color.to_color4f(), None);

            self.canvas.draw_rrect(client_rrect, &paint);
        }

        let border_is_hidden = matches!(
            style.border_style.top,
            Some(BorderStyle::None) | Some(BorderStyle::Hidden)
        );

        if !border_is_hidden {
            let border_width = style.border_width.resolved();
            let stroke_width_px = border_width
                .top
                .to_px()
                .max(border_width.right.to_px())
                .max(border_width.bottom.to_px())
                .max(border_width.left.to_px());

            if stroke_width_px > 0.0 {
                let color = style.border_color.top.unwrap_or(Rgba {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255,
                });

                let mut paint = Paint::new(color.to_color4f(), None);
                paint.set_style(skia_safe::paint::Style::Stroke);
                paint.set_stroke_width(stroke_width_px as f32);
                paint.set_anti_alias(true);
                self.canvas.draw_rrect(client_rrect, &paint);
            }
        }

        // Draw the node's text if it has any
        if let Some(text) = &node.text {
            let text_color = style.color.unwrap_or(Rgba {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            });

            let mut paint = Paint::new(text_color.to_color4f(), None);
            paint.set_anti_alias(true);

            let padding = style.padding.resolved();
            let x = (node.bounds.x + padding.left.to_px()) as f32;

            let font_spec = FontSpec::from_style(style);
            let font = SkiaTextMeasurer::make_font(&font_spec);
            let (_scale, metrics) = font.metrics();
            let baseline_y =
                (node.bounds.y + padding.top.to_px() + (-metrics.ascent as f64)) as f32;

            self.canvas.draw_str(text, (x, baseline_y), &font, &paint);
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
