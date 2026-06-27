use crate::{
    layout::{Rect, RenderNode},
    style::Overflow,
    Id,
};

pub const THUMB_THICKNESS: f64 = 8.0;
pub const THUMB_MIN_LENGTH: f64 = 18.0;
pub const THUMB_INSET: f64 = 2.0;
pub const THUMB_CORNER_RADIUS: f32 = 4.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScrollbarAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug)]
pub struct ScrollbarThumbGeometry {
    pub container_id: Id,
    pub axis: ScrollbarAxis,
    pub rect: Rect,
    pub clip_rect: Rect,
    pub visible_clip: Option<Rect>,
    pub track_length: f64,
    pub thumb_length: f64,
    pub max_scroll: f64,
}

pub fn collect_scrollbar_thumbs(root: &RenderNode) -> Vec<ScrollbarThumbGeometry> {
    let mut out = Vec::new();
    collect_scrollbar_thumbs_rec(root, None, &mut out);
    out
}

pub fn point_hits_thumb(thumb: &ScrollbarThumbGeometry, x: f64, y: f64) -> bool {
    if !thumb.rect.contains_point(x, y) {
        return false;
    }

    if let Some(clip) = thumb.visible_clip {
        return clip.contains_point(x, y);
    }

    true
}

fn collect_scrollbar_thumbs_rec(
    node: &RenderNode,
    inherited_clip: Option<Rect>,
    out: &mut Vec<ScrollbarThumbGeometry>,
) {
    out.extend(scrollbar_thumbs_for_node(node, inherited_clip));

    let descendant_clip = descendant_clip_for_node(node, inherited_clip);
    for child in &node.children {
        collect_scrollbar_thumbs_rec(child, descendant_clip, out);
    }
}

fn scrollbar_thumbs_for_node(
    node: &RenderNode,
    inherited_clip: Option<Rect>,
) -> Vec<ScrollbarThumbGeometry> {
    if !matches!(
        node.style.overflow,
        Some(Overflow::Scroll) | Some(Overflow::Auto)
    ) {
        return Vec::new();
    }

    if node.children.is_empty() {
        return Vec::new();
    }

    let border = node.style.border_width.resolved();
    let clip_rect = Rect {
        x: node.bounds.x + border.left.to_px(),
        y: node.bounds.y + border.top.to_px(),
        width: (node.bounds.width - border.left.to_px() - border.right.to_px()).max(0.0),
        height: (node.bounds.height - border.top.to_px() - border.bottom.to_px()).max(0.0),
    };

    if clip_rect.width <= 0.0 || clip_rect.height <= 0.0 {
        return Vec::new();
    }

    let visible_clip = match inherited_clip {
        Some(clip) => clip.intersect(&clip_rect),
        None => Some(clip_rect),
    };

    if visible_clip.is_none() {
        return Vec::new();
    }

    let mut content_min_x = f64::INFINITY;
    let mut content_min_y = f64::INFINITY;
    let mut content_max_x = f64::NEG_INFINITY;
    let mut content_max_y = f64::NEG_INFINITY;

    for child in &node.children {
        content_min_x = content_min_x.min(child.bounds.x);
        content_min_y = content_min_y.min(child.bounds.y);
        content_max_x = content_max_x.max(child.bounds.x + child.bounds.width);
        content_max_y = content_max_y.max(child.bounds.y + child.bounds.height);
    }

    if !content_min_x.is_finite()
        || !content_min_y.is_finite()
        || !content_max_x.is_finite()
        || !content_max_y.is_finite()
    {
        return Vec::new();
    }

    let content_w = (content_max_x - content_min_x).max(0.0);
    let content_h = (content_max_y - content_min_y).max(0.0);

    let has_h_scroll = content_w > clip_rect.width + 0.5;
    let has_v_scroll = content_h > clip_rect.height + 0.5;
    if !has_h_scroll && !has_v_scroll {
        return Vec::new();
    }

    let mut thumbs = Vec::new();

    if has_v_scroll {
        let max_scroll = (content_h - clip_rect.height).max(0.0);
        let current_scroll = (clip_rect.y - content_min_y).clamp(0.0, max_scroll);

        let thumb_length = ((clip_rect.height * clip_rect.height) / content_h)
            .clamp(THUMB_MIN_LENGTH, clip_rect.height);
        let track_length = clip_rect.height;
        let travel = (track_length - thumb_length).max(0.0);
        let thumb_offset = if max_scroll > 0.0 && travel > 0.0 {
            (current_scroll / max_scroll) * travel
        } else {
            0.0
        };

        let x2 = clip_rect.x + clip_rect.width - THUMB_INSET;
        let x1 = x2 - THUMB_THICKNESS;
        let y1 = clip_rect.y + thumb_offset;
        let y2 = y1 + thumb_length;

        thumbs.push(ScrollbarThumbGeometry {
            container_id: node.id,
            axis: ScrollbarAxis::Vertical,
            rect: Rect {
                x: x1,
                y: y1,
                width: (x2 - x1).max(0.0),
                height: (y2 - y1).max(0.0),
            },
            clip_rect,
            visible_clip,
            track_length,
            thumb_length,
            max_scroll,
        });
    }

    if has_h_scroll {
        let max_scroll = (content_w - clip_rect.width).max(0.0);
        let current_scroll = (clip_rect.x - content_min_x).clamp(0.0, max_scroll);

        let thumb_length = ((clip_rect.width * clip_rect.width) / content_w)
            .clamp(THUMB_MIN_LENGTH, clip_rect.width);
        let track_length = clip_rect.width;
        let travel = (track_length - thumb_length).max(0.0);
        let thumb_offset = if max_scroll > 0.0 && travel > 0.0 {
            (current_scroll / max_scroll) * travel
        } else {
            0.0
        };

        let y2 = clip_rect.y + clip_rect.height - THUMB_INSET;
        let y1 = y2 - THUMB_THICKNESS;
        let x1 = clip_rect.x + thumb_offset;
        let x2 = x1 + thumb_length;

        thumbs.push(ScrollbarThumbGeometry {
            container_id: node.id,
            axis: ScrollbarAxis::Horizontal,
            rect: Rect {
                x: x1,
                y: y1,
                width: (x2 - x1).max(0.0),
                height: (y2 - y1).max(0.0),
            },
            clip_rect,
            visible_clip,
            track_length,
            thumb_length,
            max_scroll,
        });
    }

    thumbs
}

fn descendant_clip_for_node(node: &RenderNode, inherited_clip: Option<Rect>) -> Option<Rect> {
    if matches!(
        node.style.overflow,
        Some(Overflow::Hidden) | Some(Overflow::Scroll) | Some(Overflow::Auto)
    ) {
        let border = node.style.border_width.resolved();
        let padding_box = Rect {
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
