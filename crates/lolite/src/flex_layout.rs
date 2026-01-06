use crate::layout::{LayoutContext, Node};
use crate::style::{AlignItems, AlignSelf, FlexDirection, FlexWrap, JustifyContent, Length, Style};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
pub struct FlexLayoutEngine;

impl FlexLayoutEngine {
    pub fn new() -> Self {
        Self
    }

    /// Runs a simplified flex layout.
    ///
    /// This is intentionally structured to follow the spec step-by-step over time.
    /// Currently, it implements the §9.1 “Initial Setup” anonymous flex item generation
    /// (in a limited form, due to the lack of explicit DOM/text node typing in the engine).
    pub fn layout_flex_children(
        &self,
        container: Rc<RefCell<Node>>,
        container_style: &Style,
        ctx: &LayoutContext,
    ) {
        // === §9.1 Initial Setup ===
        // Generate anonymous flex items as described in §4 Flex Items.
        //
        // Spec note: each in-flow child becomes a flex item, and each child text sequence is
        // wrapped in an anonymous block container flex item (and whitespace-only sequences are not rendered).
        //
        // TODO: When the engine distinguishes element nodes vs text nodes and supports true
        // "text sequences", implement proper anonymous flex item wrappers.

        let direction = container_style.flex_direction.unwrap_or(FlexDirection::Row);
        let wrap = container_style.flex_wrap.unwrap_or(FlexWrap::NoWrap);
        let justify_content = container_style
            .justify_content
            .unwrap_or(JustifyContent::FlexStart);
        let align_items = container_style.align_items.unwrap_or(AlignItems::Stretch);

        let (container_x, container_y, container_main, container_cross) = {
            let b = container.borrow().layout.bounds;
            match direction {
                FlexDirection::Row | FlexDirection::RowReverse => (b.x, b.y, b.width, b.height),
                FlexDirection::Column | FlexDirection::ColumnReverse => {
                    (b.x, b.y, b.height, b.width)
                }
            }
        };

        // === §9.2 Line Length Determination ===
        // §9.2 #2 Determine the available main and cross space for the flex items.
        // For each dimension:
        // - If that dimension of the flex container’s content box is a definite size, use that.
        // - Else if being sized under a min/max-content constraint, use that constraint. (TODO)
        // - Else subtract the flex container’s margin/border/padding from the space available
        //   to the flex container in that dimension.
        //
        // Where the definite size is determined:
        // `is_definite_container_content_box_size_*()` below is our current notion of
        // “definite” (right now: explicit px sizes only).
        let available_main =
            determine_available_space(container_main, container_style, &direction, Axis::Main);
        let available_cross =
            determine_available_space(container_cross, container_style, &direction, Axis::Cross);

        let (row_gap_px, column_gap_px) = gaps_px(container_style);
        let (main_gap_px, cross_gap_px) = match direction {
            FlexDirection::Row | FlexDirection::RowReverse => (column_gap_px, row_gap_px),
            FlexDirection::Column | FlexDirection::ColumnReverse => (row_gap_px, column_gap_px),
        };

        // Collect children, applying the "anonymous flex item" rules as best as we can.
        // In this engine, nodes are not typed; we treat a "text node" as:
        // - has `text: Some`,
        // - has no attributes,
        // - has no children.
        let mut children: Vec<Rc<RefCell<Node>>> = {
            let c = container.borrow();
            c.children.clone()
        };

        // Apply 'order' if present.
        children.sort_by_key(|child| {
            let style = resolve_style(child, ctx, container_style);
            style.order.unwrap_or(0)
        });

        let mut items: Vec<FlexItem> = Vec::new();
        for child in children {
            let is_text_node_guess = {
                let child_borrow = child.borrow();
                child_borrow.text.is_some()
                    && child_borrow.attributes.is_empty()
                    && child_borrow.children.is_empty()
            };

            if is_text_node_guess {
                let text = child.borrow().text.clone().unwrap_or_default();
                if text.trim().is_empty() {
                    // Whitespace-only child text sequences are not rendered.
                    continue;
                }
            }

            let style = resolve_style(&child, ctx, container_style);
            // NOTE: This currently approximates §9.2 #3 “Determine the flex base size and
            // hypothetical main size of each item”.
            //
            // Where flex-basis will later be handled:
            // `base_sizes_for_item()` currently applies `flex-basis` directly, but the spec’s
            // detailed cases (definite basis vs content-based basis, etc.) will replace this.
            //
            // Where aspect ratio will later be handled:
            // The spec has cases where an item’s preferred/intrinsic aspect ratio affects its
            // flex base size (see §9.2 #3). Lolite does not model aspect ratio yet.
            let (base_main, base_cross) = base_sizes_for_item(&child, &style, &direction, ctx);

            items.push(FlexItem {
                node: child,
                style,
                base_main,
                base_cross,
                final_main: base_main,
                final_cross: base_cross,
            });
        }

        if items.is_empty() {
            return;
        }

        // Form flex lines.
        let mut lines: Vec<Vec<usize>> = Vec::new();
        let mut current: Vec<usize> = Vec::new();
        let mut current_used_main = 0.0;

        let can_wrap = matches!(wrap, FlexWrap::Wrap | FlexWrap::WrapReverse);

        for (index, item) in items.iter().enumerate() {
            let additional_gap = if current.is_empty() { 0.0 } else { main_gap_px };
            let candidate_used = current_used_main + additional_gap + item.base_main;

            let should_wrap = can_wrap && !current.is_empty() && candidate_used > available_main;
            if should_wrap {
                lines.push(current);
                current = Vec::new();
                current_used_main = 0.0;
            }

            let gap = if current.is_empty() { 0.0 } else { main_gap_px };
            current_used_main += gap + item.base_main;
            current.push(index);
        }
        if !current.is_empty() {
            lines.push(current);
        }

        // Layout each line.
        let mut line_cross_offset = 0.0;
        let is_single_line = lines.len() == 1;

        for line in lines {
            // Resolve flexing within the line.
            let total_base_main = line.iter().enumerate().fold(0.0, |acc, (pos, idx)| {
                let gap = if pos > 0 { main_gap_px } else { 0.0 };
                acc + gap + items[*idx].base_main
            });

            let free_space = available_main - total_base_main;
            if free_space > 0.0 {
                let total_grow: f64 = line
                    .iter()
                    .map(|idx| items[*idx].style.flex_grow.unwrap_or(0.0))
                    .sum();

                if total_grow > 0.0 {
                    for idx in &line {
                        let grow = items[*idx].style.flex_grow.unwrap_or(0.0);
                        items[*idx].final_main =
                            items[*idx].base_main + (free_space * (grow / total_grow));
                    }
                }
            } else if free_space < 0.0 {
                let shrink_needed = -free_space;
                let weights: Vec<f64> = line
                    .iter()
                    .map(|idx| {
                        // In this codebase/tests, unspecified flex-shrink means "don't shrink".
                        let shrink = items[*idx].style.flex_shrink.unwrap_or(0.0);
                        shrink * items[*idx].base_main
                    })
                    .collect();

                let total_weight: f64 = weights.iter().sum();
                if total_weight > 0.0 {
                    for (i, idx) in line.iter().enumerate() {
                        let weight = weights[i];
                        items[*idx].final_main =
                            items[*idx].base_main - (shrink_needed * (weight / total_weight));
                    }
                }
            }

            // Determine line cross size.
            let mut line_cross_size: f64 = 0.0;
            for idx in &line {
                line_cross_size = line_cross_size.max(items[*idx].final_cross);
            }

            // Single-line definite cross size behavior (spec lives in §9.4, but it is a
            // necessary precondition for nested flex sizing to match expectations).
            // If the flex container is single-line and has a definite cross size,
            // the line’s cross size is the container’s inner cross size.
            if is_single_line
                && is_definite_container_content_box_size(container_style, &direction, Axis::Cross)
            {
                line_cross_size = available_cross;
            }

            // Apply align-items (and align-self) in the cross axis.
            for idx in &line {
                let align = match items[*idx]
                    .style
                    .align_self
                    .as_ref()
                    .unwrap_or(&AlignSelf::Auto)
                {
                    AlignSelf::Auto => align_items.clone(),
                    AlignSelf::FlexStart => AlignItems::FlexStart,
                    AlignSelf::FlexEnd => AlignItems::FlexEnd,
                    AlignSelf::Center => AlignItems::Center,
                    AlignSelf::Baseline => AlignItems::Baseline,
                    AlignSelf::Stretch => AlignItems::Stretch,
                };

                if matches!(align, AlignItems::Stretch)
                    && cross_size_is_auto(&items[*idx].style, &direction)
                {
                    items[*idx].final_cross = line_cross_size;
                }
            }

            // Recompute line used main after flexing.
            let line_used_main = line.iter().enumerate().fold(0.0, |acc, (pos, idx)| {
                let gap = if pos > 0 { main_gap_px } else { 0.0 };
                acc + gap + items[*idx].final_main
            });

            let leftover_for_justify = (available_main - line_used_main).max(0.0);
            let (start_offset, between_gap) = justify_offsets(
                &justify_content,
                &direction,
                leftover_for_justify,
                main_gap_px,
                line.len(),
            );

            let mut cursor_main = start_offset;
            for (pos, idx) in line.iter().enumerate() {
                if pos > 0 {
                    cursor_main += between_gap;
                }

                let item = &items[*idx];
                let cross_pos = match align_items {
                    AlignItems::FlexStart | AlignItems::Baseline | AlignItems::Stretch => {
                        line_cross_offset
                    }
                    AlignItems::FlexEnd => line_cross_offset + (line_cross_size - item.final_cross),
                    AlignItems::Center => {
                        line_cross_offset + (line_cross_size - item.final_cross) / 2.0
                    }
                };

                let (x, y, w, h) = match direction {
                    FlexDirection::Row | FlexDirection::RowReverse => (
                        container_x + cursor_main,
                        container_y + cross_pos,
                        item.final_main,
                        item.final_cross,
                    ),
                    FlexDirection::Column | FlexDirection::ColumnReverse => (
                        container_x + cross_pos,
                        container_y + cursor_main,
                        item.final_cross,
                        item.final_main,
                    ),
                };

                // Apply the computed (post-flexing) used size to the item.
                // This is important for nested flex containers: their own “available space”
                // (§9.2 #2) depends on the size assigned by the parent flex formatting context.
                {
                    let mut node_borrow = item.node.borrow_mut();
                    node_borrow.layout.bounds.x = x;
                    node_borrow.layout.bounds.y = y;
                    node_borrow.layout.bounds.width = w;
                    node_borrow.layout.bounds.height = h;
                    node_borrow.layout.style = std::sync::Arc::new(item.style.clone());
                }

                // Layout the child subtree (so nested flex containers work).
                // LayoutContext now respects these pre-set sizes for container nodes.
                ctx.layout_node(item.node.clone(), x, y);

                // Override the computed size after layout.
                // This is required for leaf nodes: LayoutContext will size leaves from their
                // style (e.g. height: 40px), but flexing (grow/shrink) produces a used size
                // that must win.
                {
                    let mut node_borrow = item.node.borrow_mut();
                    node_borrow.layout.bounds.width = w;
                    node_borrow.layout.bounds.height = h;
                    node_borrow.layout.style = std::sync::Arc::new(item.style.clone());
                }

                cursor_main += item.final_main;
            }

            line_cross_offset += line_cross_size + cross_gap_px;
        }
    }
}

#[derive(Clone)]
struct FlexItem {
    node: Rc<RefCell<Node>>,
    style: Style,
    base_main: f64,
    base_cross: f64,
    final_main: f64,
    final_cross: f64,
}

fn base_sizes_for_item(
    node: &Rc<RefCell<Node>>,
    style: &Style,
    direction: &FlexDirection,
    ctx: &LayoutContext,
) -> (f64, f64) {
    // Where flex-basis will later be handled: this function is the current stand-in for
    // §9.2 #3 “flex base size / hypothetical main size” rules.

    let width_opt = style.width.as_ref().map(|l| l.to_px()).filter(|v| *v > 0.0);
    let height_opt = style
        .height
        .as_ref()
        .map(|l| l.to_px())
        .filter(|v| *v > 0.0);

    // TODO handle proper size
    let width = width_opt.unwrap_or(100.0);
    let height = height_opt.unwrap_or(30.0);

    let (main_from_size, cross_from_size) = match direction {
        FlexDirection::Row | FlexDirection::RowReverse => (width, height),
        FlexDirection::Column | FlexDirection::ColumnReverse => (height, width),
    };

    let mut main = match style.flex_basis.as_ref() {
        Some(Length::Px(px)) => *px,
        Some(Length::Auto) => main_from_size,
        Some(other) => other.to_px(),
        None => main_from_size,
    };

    // If the item is itself a container and has no explicit main size, approximate
    // shrink-to-fit by looking at its children’s fixed sizes.
    // This is a pragmatic bridge until we implement the full intrinsic sizing path.
    let is_container = !node.borrow().children.is_empty();
    let has_explicit_main = match direction {
        FlexDirection::Row | FlexDirection::RowReverse => {
            matches!(style.width, Some(Length::Px(_)))
        }
        FlexDirection::Column | FlexDirection::ColumnReverse => {
            matches!(style.height, Some(Length::Px(_)))
        }
    };
    if is_container && !has_explicit_main && style.flex_basis.is_none() {
        // If the main size is currently coming from our hardcoded default, prefer
        // a child-derived intrinsic size (this is needed for shrink-to-fit flex items).
        let main_was_default = match direction {
            FlexDirection::Row | FlexDirection::RowReverse => width_opt.is_none(),
            FlexDirection::Column | FlexDirection::ColumnReverse => height_opt.is_none(),
        };

        let intrinsic = intrinsic_main_from_children(node, direction, ctx, style);
        if intrinsic > 0.0 && main_was_default {
            main = intrinsic;
        }
    }

    (main, cross_from_size)
}

#[derive(Clone, Copy)]
enum Axis {
    Main,
    Cross,
}

fn is_definite_container_content_box_size(
    style: &Style,
    direction: &FlexDirection,
    axis: Axis,
) -> bool {
    match (direction, axis) {
        (FlexDirection::Row | FlexDirection::RowReverse, Axis::Main) => {
            !matches!(style.width, Some(Length::Auto))
        }
        (FlexDirection::Row | FlexDirection::RowReverse, Axis::Cross) => {
            !matches!(style.height, Some(Length::Auto))
        }
        (FlexDirection::Column | FlexDirection::ColumnReverse, Axis::Main) => {
            !matches!(style.height, Some(Length::Auto))
        }
        (FlexDirection::Column | FlexDirection::ColumnReverse, Axis::Cross) => {
            !matches!(style.width, Some(Length::Auto))
        }
    }
}

fn determine_available_space(
    container_axis_size: f64,
    style: &Style,
    direction: &FlexDirection,
    axis: Axis,
) -> f64 {
    // §9.2 #2
    if is_definite_container_content_box_size(style, direction, axis) {
        return container_axis_size;
    }

    // TODO: If the flex container is being sized under a min/max-content constraint,
    // the available space is that constraint. Lolite currently has no constraint plumbing.

    // Otherwise, subtract margin/border/padding from the space available to the container.
    // In this engine, `container_axis_size` is already the size we’re laying out into; we can
    // at least account for padding/border to approximate the inner available space.
    let padding = axis_padding_sum_px(style, direction, axis);
    let border = axis_border_sum_px(style, direction, axis);

    (container_axis_size - padding - border).max(0.0)
}

fn axis_padding_sum_px(style: &Style, direction: &FlexDirection, axis: Axis) -> f64 {
    let Some(p) = style.padding.as_ref() else {
        return 0.0;
    };

    match (direction, axis) {
        (FlexDirection::Row | FlexDirection::RowReverse, Axis::Main)
        | (FlexDirection::Column | FlexDirection::ColumnReverse, Axis::Cross) => {
            p.left.to_px() + p.right.to_px()
        }
        (FlexDirection::Row | FlexDirection::RowReverse, Axis::Cross)
        | (FlexDirection::Column | FlexDirection::ColumnReverse, Axis::Main) => {
            p.top.to_px() + p.bottom.to_px()
        }
    }
}

fn axis_border_sum_px(style: &Style, _direction: &FlexDirection, _axis: Axis) -> f64 {
    // Lolite currently models a single uniform border width.
    // TODO should depend on box-sizing?
    style.border_width.map(|w| w.to_px() * 2.0).unwrap_or(0.0)
}

fn intrinsic_main_from_children(
    node: &Rc<RefCell<Node>>,
    parent_direction: &FlexDirection,
    ctx: &LayoutContext,
    fallback: &Style,
) -> f64 {
    // Best-effort intrinsic main size used for shrink-to-fit containers.
    // We intentionally keep this conservative (max of child fixed sizes), since Lolite
    // does not yet implement min/max-content constraints or full intrinsic sizing.

    let children = node.borrow().children.clone();
    if children.is_empty() {
        return 0.0;
    }

    let is_row_main = matches!(
        parent_direction,
        FlexDirection::Row | FlexDirection::RowReverse
    );

    children
        .iter()
        .map(|c| {
            let s = resolve_style(c, ctx, fallback);
            if is_row_main {
                s.width.as_ref().map(|l| l.to_px()).unwrap_or(100.0)
            } else {
                s.height.as_ref().map(|l| l.to_px()).unwrap_or(30.0)
            }
        })
        .fold(0.0, f64::max)
}

fn cross_size_is_auto(style: &Style, direction: &FlexDirection) -> bool {
    match direction {
        FlexDirection::Row | FlexDirection::RowReverse => style.height.is_none(),
        FlexDirection::Column | FlexDirection::ColumnReverse => style.width.is_none(),
    }
}

fn gaps_px(style: &Style) -> (f64, f64) {
    if let Some(gap) = style.gap.as_ref() {
        let px = gap.to_px();
        return (px, px);
    }

    let row_gap = style.row_gap.as_ref().map(|l| l.to_px()).unwrap_or(0.0);
    let col_gap = style.column_gap.as_ref().map(|l| l.to_px()).unwrap_or(0.0);
    (row_gap, col_gap)
}

fn justify_offsets(
    justify: &JustifyContent,
    direction: &FlexDirection,
    leftover: f64,
    base_gap: f64,
    item_count: usize,
) -> (f64, f64) {
    if item_count == 0 {
        return (0.0, base_gap);
    }

    // Reverse directions flip the meaning of flex-start/flex-end.
    let is_reverse = matches!(
        direction,
        FlexDirection::RowReverse | FlexDirection::ColumnReverse
    );
    let justify = match (is_reverse, justify) {
        (true, JustifyContent::FlexStart) => JustifyContent::FlexEnd,
        (true, JustifyContent::FlexEnd) => JustifyContent::FlexStart,
        _ => justify.clone(),
    };

    match justify {
        JustifyContent::FlexStart => (0.0, base_gap),
        JustifyContent::FlexEnd => (leftover, base_gap),
        JustifyContent::Center => (leftover / 2.0, base_gap),
        JustifyContent::SpaceBetween => {
            if item_count <= 1 {
                (0.0, base_gap)
            } else {
                let extra = leftover / (item_count as f64 - 1.0);
                (0.0, base_gap + extra)
            }
        }
        JustifyContent::SpaceAround => {
            let extra = leftover / item_count as f64;
            (extra / 2.0, base_gap + extra)
        }
        JustifyContent::SpaceEvenly => {
            let extra = leftover / (item_count as f64 + 1.0);
            (extra, base_gap + extra)
        }
    }
}

fn resolve_style(node: &Rc<RefCell<Node>>, ctx: &LayoutContext, fallback: &Style) -> Style {
    let node_borrow = node.borrow();

    // Start with existing style as base.
    let mut style = node_borrow.layout.style.as_ref().clone();

    // Apply CSS rules for class selector.
    if let Some(class_attr) = node_borrow.attributes.get("class") {
        for class_name in class_attr.split_whitespace() {
            let selector = crate::style::Selector::Class(class_name.to_string());
            if let Some(rule) = ctx
                .style_sheet
                .rules
                .iter()
                .find(|rule| rule.selector == selector)
            {
                for declaration in &rule.declarations {
                    style.merge(declaration);
                }
            }
        }
    }

    // Best-effort inheritance for anonymous items.
    if node_borrow.attributes.is_empty() && node_borrow.children.is_empty() {
        style.display = fallback.display.clone();
    }

    style
}
