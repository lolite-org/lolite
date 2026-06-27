use crate::{
    layout::RenderSnapshot,
    scrollbar::{collect_scrollbar_thumbs, point_hits_thumb, ScrollbarAxis},
    Id,
};

#[derive(Clone, Copy)]
pub struct ActiveScrollbarDrag {
    pub container_id: Id,
    pub axis: ScrollbarAxis,
    pub grab_offset: f64,
}

#[derive(Clone, Copy)]
pub struct ScrollbarDragUpdate {
    pub container_id: Id,
    pub axis: ScrollbarAxis,
    pub new_scroll: f64,
}

pub fn begin_scrollbar_drag(
    snapshot: &RenderSnapshot,
    x: f64,
    y: f64,
) -> Option<ActiveScrollbarDrag> {
    let thumbs = collect_scrollbar_thumbs(&snapshot.root);
    for thumb in thumbs.iter().rev() {
        if point_hits_thumb(thumb, x, y) {
            let grab_offset = match thumb.axis {
                ScrollbarAxis::Vertical => y - thumb.rect.y,
                ScrollbarAxis::Horizontal => x - thumb.rect.x,
            }
            .clamp(0.0, thumb.thumb_length);

            return Some(ActiveScrollbarDrag {
                container_id: thumb.container_id,
                axis: thumb.axis,
                grab_offset,
            });
        }
    }

    None
}

pub fn update_scrollbar_drag(
    snapshot: &RenderSnapshot,
    active: ActiveScrollbarDrag,
    x: f64,
    y: f64,
) -> Option<ScrollbarDragUpdate> {
    let thumbs = collect_scrollbar_thumbs(&snapshot.root);
    let thumb = thumbs
        .iter()
        .rev()
        .find(|thumb| thumb.container_id == active.container_id && thumb.axis == active.axis)
        .copied()?;

    let cursor_main = match active.axis {
        ScrollbarAxis::Vertical => y,
        ScrollbarAxis::Horizontal => x,
    };
    let track_start = match active.axis {
        ScrollbarAxis::Vertical => thumb.clip_rect.y,
        ScrollbarAxis::Horizontal => thumb.clip_rect.x,
    };

    let travel = (thumb.track_length - thumb.thumb_length).max(0.0);
    let start = (cursor_main - active.grab_offset).clamp(track_start, track_start + travel);
    let offset = (start - track_start).max(0.0);

    let new_scroll = if travel > 0.0 && thumb.max_scroll > 0.0 {
        (offset / travel) * thumb.max_scroll
    } else {
        0.0
    };

    Some(ScrollbarDragUpdate {
        container_id: active.container_id,
        axis: active.axis,
        new_scroll,
    })
}
