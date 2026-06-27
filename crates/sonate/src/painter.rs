use crate::{
    layout::{Rect as LayoutRect, RenderOp, RenderSnapshot},
    style::{BorderStyle, Length, Overflow, Rgba},
    text::{FontSpec, SkiaTextMeasurer},
};
use skia_safe::{Canvas, ClipOp, Color, Color4f, Paint, Path, RRect, Rect};

pub struct Painter<'a> {
    canvas: &'a Canvas,
}

impl<'a> Painter<'a> {
    pub fn new(canvas: &'a Canvas) -> Self {
        Self { canvas }
    }

    pub fn paint(&mut self, snapshot: &RenderSnapshot) {
        self.canvas.clear(Color::WHITE);
        self.paint_ops(&snapshot.render_list);
        self.paint_scrollbars(&snapshot.root, None);
    }

    fn paint_scrollbars(
        &mut self,
        node: &crate::layout::RenderNode,
        inherited_clip: Option<LayoutRect>,
    ) {
        self.paint_scrollbar_for_node(node, inherited_clip);
        let descendant_clip = Self::descendant_clip_for_node(node, inherited_clip);
        for child in &node.children {
            self.paint_scrollbars(child, descendant_clip);
        }
    }

    fn paint_scrollbar_for_node(
        &mut self,
        node: &crate::layout::RenderNode,
        inherited_clip: Option<LayoutRect>,
    ) {
        if !matches!(
            node.style.overflow,
            Some(Overflow::Scroll) | Some(Overflow::Auto)
        ) {
            return;
        }

        if node.children.is_empty() {
            return;
        }

        let border = node.style.border_width.resolved();
        let clip_x = node.bounds.x + border.left.to_px();
        let clip_y = node.bounds.y + border.top.to_px();
        let clip_w = (node.bounds.width - border.left.to_px() - border.right.to_px()).max(0.0);
        let clip_h = (node.bounds.height - border.top.to_px() - border.bottom.to_px()).max(0.0);

        if clip_w <= 0.0 || clip_h <= 0.0 {
            return;
        }

        let mut content_min_x = f64::INFINITY;
        let mut content_min_y = f64::INFINITY;
        let mut content_max_x = f64::NEG_INFINITY;
        let mut content_max_y = f64::NEG_INFINITY;

        for child in &node.children {
            Self::accumulate_bounds(
                child,
                &mut content_min_x,
                &mut content_min_y,
                &mut content_max_x,
                &mut content_max_y,
            );
        }

        if !content_min_x.is_finite() || !content_min_y.is_finite() {
            return;
        }

        let content_w = (content_max_x - content_min_x).max(0.0);
        let content_h = (content_max_y - content_min_y).max(0.0);

        let has_h_scroll = content_w > clip_w + 0.5;
        let has_v_scroll = content_h > clip_h + 0.5;

        if !has_h_scroll && !has_v_scroll {
            return;
        }

        let has_clip = inherited_clip.is_some();
        if let Some(clip) = inherited_clip {
            self.canvas.save();
            self.canvas
                .clip_rect(Self::to_skia_rect(clip), ClipOp::Intersect, true);
        }

        let mut paint = Paint::new(Color4f::new(0.20, 0.20, 0.20, 0.55), None);
        paint.set_anti_alias(true);

        let thumb_thickness = 8.0_f32;
        let corner = 4.0_f32;
        let inset = 2.0_f32;

        if has_v_scroll {
            let max_scroll = (content_h - clip_h).max(1.0);
            let scroll = (clip_y - content_min_y).clamp(0.0, max_scroll);

            let thumb_h = ((clip_h * clip_h) / content_h).clamp(18.0, clip_h);
            let track_h = clip_h;
            let travel = (track_h - thumb_h).max(0.0);
            let thumb_offset = if travel > 0.0 {
                (scroll / max_scroll) * travel
            } else {
                0.0
            };

            let x2 = (clip_x + clip_w - inset as f64) as f32;
            let x1 = x2 - thumb_thickness;
            let y1 = (clip_y + thumb_offset) as f32;
            let y2 = (clip_y + thumb_offset + thumb_h) as f32;
            let rect = Rect::new(x1, y1, x2, y2);
            let rrect = RRect::new_rect_xy(rect, corner, corner);
            self.canvas.draw_rrect(rrect, &paint);
        }

        if has_h_scroll {
            let max_scroll = (content_w - clip_w).max(1.0);
            let scroll = (clip_x - content_min_x).clamp(0.0, max_scroll);

            let thumb_w = ((clip_w * clip_w) / content_w).clamp(18.0, clip_w);
            let track_w = clip_w;
            let travel = (track_w - thumb_w).max(0.0);
            let thumb_offset = if travel > 0.0 {
                (scroll / max_scroll) * travel
            } else {
                0.0
            };

            let y2 = (clip_y + clip_h - inset as f64) as f32;
            let y1 = y2 - thumb_thickness;
            let x1 = (clip_x + thumb_offset) as f32;
            let x2 = (clip_x + thumb_offset + thumb_w) as f32;
            let rect = Rect::new(x1, y1, x2, y2);
            let rrect = RRect::new_rect_xy(rect, corner, corner);
            self.canvas.draw_rrect(rrect, &paint);
        }

        if has_clip {
            self.canvas.restore();
        }
    }

    fn descendant_clip_for_node(
        node: &crate::layout::RenderNode,
        inherited_clip: Option<LayoutRect>,
    ) -> Option<LayoutRect> {
        if matches!(
            node.style.overflow,
            Some(Overflow::Hidden) | Some(Overflow::Scroll) | Some(Overflow::Auto)
        ) {
            let border = node.style.border_width.resolved();
            let padding_box = LayoutRect {
                x: node.bounds.x + border.left.to_px(),
                y: node.bounds.y + border.top.to_px(),
                width: (node.bounds.width - border.left.to_px() - border.right.to_px()).max(0.0),
                height: (node.bounds.height - border.top.to_px() - border.bottom.to_px()).max(0.0),
            };

            return match inherited_clip {
                Some(clip) => clip.intersect(&padding_box),
                None => Some(padding_box),
            };
        }

        inherited_clip
    }

    fn to_skia_rect(rect: LayoutRect) -> Rect {
        Rect::new(
            rect.x as f32,
            rect.y as f32,
            (rect.x + rect.width) as f32,
            (rect.y + rect.height) as f32,
        )
    }

    fn accumulate_bounds(
        node: &crate::layout::RenderNode,
        min_x: &mut f64,
        min_y: &mut f64,
        max_x: &mut f64,
        max_y: &mut f64,
    ) {
        *min_x = (*min_x).min(node.bounds.x);
        *min_y = (*min_y).min(node.bounds.y);
        *max_x = (*max_x).max(node.bounds.x + node.bounds.width);
        *max_y = (*max_y).max(node.bounds.y + node.bounds.height);

        for child in &node.children {
            Self::accumulate_bounds(child, min_x, min_y, max_x, max_y);
        }
    }

    fn paint_ops(&mut self, ops: &[RenderOp]) {
        for op in ops {
            match op {
                RenderOp::DrawBackgroundBorder(node) => self.paint_background_and_border(node),
                RenderOp::DrawText(node) => self.paint_text(node),
                RenderOp::PushClip(rect) => {
                    self.canvas.save();
                    let clip_rect = Rect::new(
                        rect.x as f32,
                        rect.y as f32,
                        (rect.x + rect.width) as f32,
                        (rect.y + rect.height) as f32,
                    );
                    self.canvas.clip_rect(clip_rect, ClipOp::Intersect, true);
                }
                RenderOp::PopClip => {
                    self.canvas.restore();
                }
            }
        }
    }

    fn paint_background_and_border(&mut self, node: &crate::layout::RenderNode) {
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
    }

    fn paint_text(&mut self, node: &crate::layout::RenderNode) {
        let style = &node.style;

        let font_spec = FontSpec::from_style(style);
        let font = SkiaTextMeasurer::make_font(&font_spec);
        let (_scale, metrics) = font.metrics();

        let padding = style.padding.resolved();
        let x = (node.bounds.x + padding.left.to_px()) as f32;
        let baseline_y = (node.bounds.y + padding.top.to_px() + (-metrics.ascent as f64)) as f32;

        if let Some(text) = &node.text {
            let text_color = style.color.unwrap_or(Rgba {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            });

            let mut paint = Paint::new(text_color.to_color4f(), None);
            paint.set_anti_alias(true);

            self.canvas.draw_str(text, (x, baseline_y), &font, &paint);
        }

        if style.widget.is_some_and(|widget| widget.is_text_input()) {
            if let Some(state) = &node.text_input_state {
                if state.focused {
                    let text = node.text.as_deref().unwrap_or("");
                    let caret_prefix: String = text.chars().take(state.caret).collect();
                    let (caret_advance, _) = font.measure_str(caret_prefix.as_str(), None);

                    let caret_x = x + caret_advance;
                    let caret_top = (baseline_y as f64 + metrics.ascent as f64) as f32;
                    let caret_bottom = (baseline_y as f64 + metrics.descent as f64) as f32;

                    let mut caret_path = Path::new();
                    caret_path.move_to((caret_x, caret_top));
                    caret_path.line_to((caret_x, caret_bottom));

                    let mut caret_paint = Paint::new(
                        style
                            .color
                            .unwrap_or(Rgba {
                                r: 0,
                                g: 0,
                                b: 0,
                                a: 255,
                            })
                            .to_color4f(),
                        None,
                    );
                    caret_paint.set_style(skia_safe::paint::Style::Stroke);
                    caret_paint.set_stroke_width(1.0);
                    caret_paint.set_anti_alias(true);
                    self.canvas.draw_path(&caret_path, &caret_paint);
                }
            }
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
