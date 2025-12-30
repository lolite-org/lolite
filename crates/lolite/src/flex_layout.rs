/*!
 * Flexbox Layout Engine
 *
 * This module contains the complete flexbox layout implementation moved from
 * the main engine for better code organization. It includes support for:
 * - All flex directions (row, column, row-reverse, column-reverse)
 * - Flex wrapping (nowrap, wrap, wrap-reverse)
 * - CSS Gap properties (gap, row-gap, column-gap)
 * - Proper coordinate calculation and child positioning
 */

use crate::{
    engine::{Engine, Node},
    style::{AlignContent, AlignItems, FlexDirection, FlexWrap, JustifyContent, Length, Style},
};
use std::{cell::RefCell, rc::Rc};

/// Flexbox layout engine that handles all flexbox positioning logic
pub struct FlexLayoutEngine;

impl FlexLayoutEngine {
    pub fn new() -> Self {
        Self
    }

    /// Calculate padding values from style
    fn calculate_padding(&self, style: &Style) -> (f64, f64, f64, f64) {
        if let Some(padding) = &style.padding {
            (
                padding.top.to_px(),
                padding.right.to_px(),
                padding.bottom.to_px(),
                padding.left.to_px(),
            )
        } else {
            (0.0, 0.0, 0.0, 0.0)
        }
    }

    /// Calculate margin values from style
    fn calculate_margin(&self, style: &Style) -> (f64, f64, f64, f64) {
        if let Some(margin) = &style.margin {
            (
                margin.top.to_px(),
                margin.right.to_px(),
                margin.bottom.to_px(),
                margin.left.to_px(),
            )
        } else {
            (0.0, 0.0, 0.0, 0.0)
        }
    }

    /// Layout children of a flex container according to flexbox rules
    /// This is the main entry point for flex layout logic
    pub fn layout_flex_children(
        &self,
        container: Rc<RefCell<Node>>,
        style: &Style,
        engine: &Engine,
    ) {
        let flex_direction = style.flex_direction.as_ref().unwrap_or(&FlexDirection::Row);
        let flex_wrap = style.flex_wrap.as_ref().unwrap_or(&FlexWrap::NoWrap);

        let container_bounds = container.borrow().layout.bounds;
        let container_x = container_bounds.x;
        let container_y = container_bounds.y;
        let container_width = container_bounds.width;
        let container_height = container_bounds.height;

        // Calculate padding to adjust the content area
        let (padding_top, padding_right, padding_bottom, padding_left) =
            self.calculate_padding(style);

        // Adjust content area for padding
        let content_x = container_x + padding_left;
        let content_y = container_y + padding_top;
        let content_width = container_width - padding_left - padding_right;
        let content_height = container_height - padding_top - padding_bottom;

        // First pass: layout all children to get their natural sizes
        let children = container.borrow().children.clone();
        for child in &children {
            // Recursively layout child first (this will apply its styles and set its dimensions)
            engine.layout_node(child.clone(), 0.0, 0.0);
        }

        // Second pass: position children based on flex direction and wrapping
        match flex_direction {
            FlexDirection::Row => {
                self.layout_row_with_wrap(
                    &children,
                    content_x,
                    content_y,
                    content_width,
                    content_height,
                    flex_wrap,
                    style,
                    engine,
                );
            }
            FlexDirection::Column => {
                self.layout_column_with_wrap(
                    &children,
                    content_x,
                    content_y,
                    content_height,
                    flex_wrap,
                    style,
                    engine,
                );
            }
            FlexDirection::RowReverse => {
                self.layout_row_reverse_with_wrap(
                    &children,
                    content_x,
                    content_y,
                    content_width,
                    flex_wrap,
                );
            }
            FlexDirection::ColumnReverse => {
                self.layout_column_reverse_with_wrap(
                    &children,
                    content_x,
                    content_y,
                    content_height,
                    flex_wrap,
                );
            }
        }
    }

    /// Layout children in a row with wrapping support and gap handling
    fn layout_row_with_wrap(
        &self,
        children: &[Rc<RefCell<Node>>],
        container_x: f64,
        container_y: f64,
        container_width: f64,
        container_height: f64,
        flex_wrap: &FlexWrap,
        style: &Style,
        engine: &Engine,
    ) {
        // Get gap values - CSS gap properties take precedence over individual gap properties
        let column_gap = if let Some(gap) = &style.gap {
            gap.to_px()
        } else if let Some(column_gap) = &style.column_gap {
            column_gap.to_px()
        } else {
            0.0
        };

        let row_gap = if let Some(gap) = &style.gap {
            gap.to_px()
        } else if let Some(row_gap) = &style.row_gap {
            row_gap.to_px()
        } else {
            0.0
        };

        match flex_wrap {
            FlexWrap::NoWrap => {
                // Apply justify-content for single line with gap support
                self.apply_justify_content_row_with_gap(
                    children,
                    container_x,
                    container_y,
                    container_width,
                    container_height,
                    style,
                    column_gap,
                );
            }
            FlexWrap::Wrap => {
                // First pass: organize children into lines, accounting for gaps
                let mut lines: Vec<Vec<Rc<RefCell<Node>>>> = Vec::new();
                let mut current_line: Vec<Rc<RefCell<Node>>> = Vec::new();
                let mut current_line_width = 0.0;

                for child in children {
                    let child_bounds = child.borrow().layout.bounds;

                    // Calculate required width including gap (if not first item in line)
                    let required_width = if current_line.is_empty() {
                        child_bounds.width
                    } else {
                        child_bounds.width + column_gap
                    };

                    // Check if this child would overflow the container width
                    if current_line_width + required_width > container_width
                        && !current_line.is_empty()
                    {
                        // Start new line
                        lines.push(current_line);
                        current_line = Vec::new();
                        current_line_width = 0.0;
                    }

                    current_line.push(child.clone());
                    current_line_width += if current_line.len() == 1 {
                        child_bounds.width
                    } else {
                        child_bounds.width + column_gap
                    };
                }

                // Add the last line if it has children
                if !current_line.is_empty() {
                    lines.push(current_line);
                }

                // Second pass: calculate line heights
                let mut line_heights: Vec<f64> = Vec::new();
                for line in &lines {
                    let line_height = line
                        .iter()
                        .map(|child| child.borrow().layout.bounds.height)
                        .fold(0.0, f64::max);
                    line_heights.push(line_height);
                }

                // Third pass: apply align-content to position lines with row gaps
                let align_content = style
                    .align_content
                    .as_ref()
                    .unwrap_or(&AlignContent::FlexStart);

                let line_start_positions = self.calculate_line_positions_with_gap(
                    &line_heights,
                    container_y,
                    container_height,
                    align_content,
                    row_gap,
                );

                // Fourth pass: position children within their lines with column gaps
                for (line_index, line) in lines.iter().enumerate() {
                    let line_y = line_start_positions[line_index];

                    // Position children horizontally within the line with gaps
                    let mut current_x = container_x;
                    for (child_index, child) in line.iter().enumerate() {
                        let child_bounds = child.borrow().layout.bounds;
                        let mut child_borrow = child.borrow_mut();

                        child_borrow.layout.bounds.x = current_x;
                        child_borrow.layout.bounds.y = line_y;

                        current_x += child_bounds.width;

                        // Add column gap after each item except the last
                        if child_index < line.len() - 1 {
                            current_x += column_gap;
                        }
                    }
                }
            }
            FlexWrap::WrapReverse => {
                // TODO: Implement wrap-reverse with gap support
                // For now, behave like wrap
                self.layout_row_with_wrap(
                    children,
                    container_x,
                    container_y,
                    container_width,
                    container_height,
                    &FlexWrap::Wrap,
                    style,
                    engine,
                );
            }
        }
    }

    /// Layout children in a column with wrapping support
    fn layout_column_with_wrap(
        &self,
        children: &[Rc<RefCell<Node>>],
        container_x: f64,
        container_y: f64,
        container_height: f64,
        flex_wrap: &FlexWrap,
        style: &Style,
        engine: &Engine,
    ) {
        // Get gap values
        let row_gap = if let Some(gap) = &style.gap {
            gap.to_px()
        } else if let Some(row_gap) = &style.row_gap {
            row_gap.to_px()
        } else {
            0.0
        };

        match flex_wrap {
            FlexWrap::NoWrap => {
                // Calculate flex sizes first - this handles flex-grow, flex-shrink, and flex-basis
                let final_heights =
                    self.calculate_flex_sizes_column(children, container_height, row_gap);

                // Update children bounds with calculated flex sizes
                for (i, child) in children.iter().enumerate() {
                    let mut child_borrow = child.borrow_mut();
                    child_borrow.layout.bounds.height = final_heights[i];
                }

                // Position children with calculated heights and margins
                let mut current_y = container_y;

                for (index, child) in children.iter().enumerate() {
                    let child_style = child.borrow().layout.style.clone();
                    let (margin_top, _margin_right, margin_bottom, margin_left) =
                        self.calculate_margin(&child_style);

                    // Position child with margins
                    let mut child_borrow = child.borrow_mut();
                    child_borrow.layout.bounds.x = container_x + margin_left;
                    child_borrow.layout.bounds.y = current_y + margin_top;

                    current_y += final_heights[index] + margin_top + margin_bottom;

                    // Add row gap after each item except the last
                    if index < children.len() - 1 {
                        current_y += row_gap;
                    }
                }
            }
            FlexWrap::Wrap => {
                let column_gap = if let Some(gap) = &style.gap {
                    gap.to_px()
                } else if let Some(column_gap) = &style.column_gap {
                    column_gap.to_px()
                } else {
                    0.0
                };

                let mut current_x = container_x;
                let mut current_y = container_y;
                let mut column_width = 0.0;

                for (index, child) in children.iter().enumerate() {
                    let child_bounds = child.borrow().layout.bounds;

                    // Calculate required height including gap (if not first item in column)
                    let required_height = if current_y == container_y {
                        child_bounds.height
                    } else {
                        child_bounds.height + row_gap
                    };

                    // Check if this child would overflow the container height
                    if current_y + required_height > container_y + container_height
                        && current_y > container_y
                    {
                        // Wrap to next column
                        current_y = container_y;
                        current_x += column_width + column_gap;
                        column_width = 0.0;
                    }

                    // Position child
                    let mut child_borrow = child.borrow_mut();
                    child_borrow.layout.bounds.x = current_x;
                    child_borrow.layout.bounds.y = current_y;

                    current_y += child_bounds.height;

                    // Add row gap after each item except the last in column
                    if index < children.len() - 1 {
                        current_y += row_gap;
                    }

                    column_width = column_width.max(child_bounds.width);
                }
            }
            FlexWrap::WrapReverse => {
                // TODO: Implement wrap-reverse with gap support
                // For now, behave like wrap
                self.layout_column_with_wrap(
                    children,
                    container_x,
                    container_y,
                    container_height,
                    &FlexWrap::Wrap,
                    style,
                    engine,
                );
            }
        }
    }

    /// Layout children in row-reverse with wrapping support
    fn layout_row_reverse_with_wrap(
        &self,
        children: &[Rc<RefCell<Node>>],
        container_x: f64,
        container_y: f64,
        container_width: f64,
        flex_wrap: &FlexWrap,
    ) {
        match flex_wrap {
            FlexWrap::NoWrap => {
                // Original nowrap behavior
                let mut current_x = container_x + container_width;
                let current_y = container_y;

                for child in children.iter().rev() {
                    let child_bounds = child.borrow().layout.bounds;
                    current_x -= child_bounds.width;

                    // Position child
                    let mut child_borrow = child.borrow_mut();
                    child_borrow.layout.bounds.x = current_x;
                    child_borrow.layout.bounds.y = current_y;
                }
            }
            FlexWrap::Wrap | FlexWrap::WrapReverse => {
                // TODO: Implement wrapping for row-reverse with gap support
                // For now, use nowrap behavior
                self.layout_row_reverse_with_wrap(
                    children,
                    container_x,
                    container_y,
                    container_width,
                    &FlexWrap::NoWrap,
                );
            }
        }
    }

    /// Layout children in column-reverse with wrapping support
    fn layout_column_reverse_with_wrap(
        &self,
        children: &[Rc<RefCell<Node>>],
        container_x: f64,
        container_y: f64,
        container_height: f64,
        flex_wrap: &FlexWrap,
    ) {
        match flex_wrap {
            FlexWrap::NoWrap => {
                // Original nowrap behavior
                let current_x = container_x;
                let mut current_y = container_y + container_height;

                for child in children.iter().rev() {
                    let child_bounds = child.borrow().layout.bounds;
                    current_y -= child_bounds.height;

                    // Position child
                    let mut child_borrow = child.borrow_mut();
                    child_borrow.layout.bounds.x = current_x;
                    child_borrow.layout.bounds.y = current_y;
                }
            }
            FlexWrap::Wrap | FlexWrap::WrapReverse => {
                // TODO: Implement wrapping for column-reverse with gap support
                // For now, use nowrap behavior
                self.layout_column_reverse_with_wrap(
                    children,
                    container_x,
                    container_y,
                    container_height,
                    &FlexWrap::NoWrap,
                );
            }
        }
    }

    /// Apply flex-grow, flex-shrink, and flex-basis to calculate final sizes for row direction
    fn calculate_flex_sizes_row(
        &self,
        children: &[Rc<RefCell<Node>>],
        container_width: f64,
        column_gap: f64,
    ) -> Vec<f64> {
        let mut final_widths = Vec::new();

        // First, calculate the base sizes (flex-basis or width)
        let mut base_sizes = Vec::new();
        let mut flex_grows = Vec::new();
        let mut flex_shrinks = Vec::new();

        for child in children {
            let child_style = child.borrow().layout.style.clone();

            // Get flex-basis or fall back to width
            let base_size = if let Some(flex_basis) = &child_style.flex_basis {
                match flex_basis {
                    Length::Auto => {
                        // Use width if available, otherwise use current bounds width
                        child_style
                            .width
                            .as_ref()
                            .map(|w| w.to_px())
                            .unwrap_or(child.borrow().layout.bounds.width)
                    }
                    _ => flex_basis.to_px(),
                }
            } else {
                // No flex-basis, use width or current bounds
                child_style
                    .width
                    .as_ref()
                    .map(|w| w.to_px())
                    .unwrap_or(child.borrow().layout.bounds.width)
            };

            base_sizes.push(base_size);
            flex_grows.push(child_style.flex_grow.unwrap_or(0.0));
            flex_shrinks.push(child_style.flex_shrink.unwrap_or(1.0));
        }

        // Calculate total base size and gaps
        let total_base_size: f64 = base_sizes.iter().sum();
        let total_gap_size = if children.len() > 1 {
            column_gap * (children.len() - 1) as f64
        } else {
            0.0
        };
        let total_content_size = total_base_size + total_gap_size;

        // Check if any flex properties are explicitly set
        let has_explicit_flex_properties = children.iter().any(|child| {
            let style = child.borrow().layout.style.clone();
            style.flex_grow.is_some() || style.flex_shrink.is_some() || style.flex_basis.is_some()
        });

        // Determine if we need to grow or shrink
        let free_space = container_width - total_content_size;

        if has_explicit_flex_properties {
            if free_space > 0.0 {
                // We have extra space - apply flex-grow
                let total_grow: f64 = flex_grows.iter().sum();

                if total_grow > 0.0 {
                    for (i, &base_size) in base_sizes.iter().enumerate() {
                        let grow_ratio = flex_grows[i] / total_grow;
                        final_widths.push(base_size + (free_space * grow_ratio));
                    }
                } else {
                    // No flex-grow values, use base sizes
                    final_widths = base_sizes;
                }
            } else if free_space < 0.0 {
                // We need to shrink - apply flex-shrink
                let overflow = -free_space;
                let mut weighted_shrink_sum = 0.0;

                // Calculate weighted shrink sum (flex-shrink * base-size)
                for (i, &base_size) in base_sizes.iter().enumerate() {
                    weighted_shrink_sum += flex_shrinks[i] * base_size;
                }

                if weighted_shrink_sum > 0.0 {
                    for (i, &base_size) in base_sizes.iter().enumerate() {
                        let shrink_ratio = (flex_shrinks[i] * base_size) / weighted_shrink_sum;
                        let shrink_amount = overflow * shrink_ratio;
                        final_widths.push((base_size - shrink_amount).max(0.0));
                    }
                } else {
                    // No flex-shrink values, use base sizes (may overflow)
                    final_widths = base_sizes;
                }
            } else {
                // Perfect fit - use base sizes
                final_widths = base_sizes;
            }
        } else {
            // No explicit flex properties - use base sizes (legacy behavior)
            final_widths = base_sizes;
        }

        final_widths
    }

    /// Apply flex-grow, flex-shrink, and flex-basis to calculate final sizes for column direction
    fn calculate_flex_sizes_column(
        &self,
        children: &[Rc<RefCell<Node>>],
        container_height: f64,
        row_gap: f64,
    ) -> Vec<f64> {
        let mut final_heights = Vec::new();

        // First, calculate the base sizes (flex-basis or height)
        let mut base_sizes = Vec::new();
        let mut flex_grows = Vec::new();
        let mut flex_shrinks = Vec::new();

        for child in children {
            let child_style = child.borrow().layout.style.clone();

            // Get flex-basis or fall back to height
            let base_size = if let Some(flex_basis) = &child_style.flex_basis {
                match flex_basis {
                    Length::Auto => {
                        // Use height if available, otherwise use current bounds height
                        child_style
                            .height
                            .as_ref()
                            .map(|h| h.to_px())
                            .unwrap_or(child.borrow().layout.bounds.height)
                    }
                    _ => flex_basis.to_px(),
                }
            } else {
                // No flex-basis, use height or current bounds
                child_style
                    .height
                    .as_ref()
                    .map(|h| h.to_px())
                    .unwrap_or(child.borrow().layout.bounds.height)
            };

            base_sizes.push(base_size);
            flex_grows.push(child_style.flex_grow.unwrap_or(0.0));
            flex_shrinks.push(child_style.flex_shrink.unwrap_or(1.0));
        }

        // Calculate total base size and gaps
        let total_base_size: f64 = base_sizes.iter().sum();
        let total_gap_size = if children.len() > 1 {
            row_gap * (children.len() - 1) as f64
        } else {
            0.0
        };
        let total_content_size = total_base_size + total_gap_size;

        // Check if any flex properties are explicitly set
        let has_explicit_flex_properties = children.iter().any(|child| {
            let style = child.borrow().layout.style.clone();
            style.flex_grow.is_some() || style.flex_shrink.is_some() || style.flex_basis.is_some()
        });

        // Determine if we need to grow or shrink
        let free_space = container_height - total_content_size;

        if has_explicit_flex_properties {
            if free_space > 0.0 {
                // We have extra space - apply flex-grow
                let total_grow: f64 = flex_grows.iter().sum();

                if total_grow > 0.0 {
                    for (i, &base_size) in base_sizes.iter().enumerate() {
                        let grow_ratio = flex_grows[i] / total_grow;
                        final_heights.push(base_size + (free_space * grow_ratio));
                    }
                } else {
                    // No flex-grow values, use base sizes
                    final_heights = base_sizes;
                }
            } else if free_space < 0.0 {
                // We need to shrink - apply flex-shrink
                let overflow = -free_space;
                let mut weighted_shrink_sum = 0.0;

                // Calculate weighted shrink sum (flex-shrink * base-size)
                for (i, &base_size) in base_sizes.iter().enumerate() {
                    weighted_shrink_sum += flex_shrinks[i] * base_size;
                }

                if weighted_shrink_sum > 0.0 {
                    for (i, &base_size) in base_sizes.iter().enumerate() {
                        let shrink_ratio = (flex_shrinks[i] * base_size) / weighted_shrink_sum;
                        let shrink_amount = overflow * shrink_ratio;
                        final_heights.push((base_size - shrink_amount).max(0.0));
                    }
                } else {
                    // No flex-shrink values, use base sizes (may overflow)
                    final_heights = base_sizes;
                }
            } else {
                // Perfect fit - use base sizes
                final_heights = base_sizes;
            }
        } else {
            // No explicit flex properties - use base sizes (legacy behavior)
            final_heights = base_sizes;
        }

        final_heights
    }

    /// Apply justify-content positioning for row direction with gap support
    fn apply_justify_content_row_with_gap(
        &self,
        children: &[Rc<RefCell<Node>>],
        container_x: f64,
        container_y: f64,
        container_width: f64,
        container_height: f64,
        style: &Style,
        column_gap: f64,
    ) {
        let justify_content = style
            .justify_content
            .as_ref()
            .unwrap_or(&JustifyContent::FlexStart);
        let align_items = style.align_items.as_ref().unwrap_or(&AlignItems::FlexStart);

        // Calculate flex sizes first - this handles flex-grow, flex-shrink, and flex-basis
        let final_widths = self.calculate_flex_sizes_row(children, container_width, column_gap);

        // Update children bounds with calculated flex sizes
        for (i, child) in children.iter().enumerate() {
            let mut child_borrow = child.borrow_mut();
            child_borrow.layout.bounds.width = final_widths[i];
        }

        // Calculate total width and free space after flex sizing
        let total_child_width: f64 = final_widths.iter().sum();
        let total_gap_width = if children.len() > 1 {
            column_gap * (children.len() - 1) as f64
        } else {
            0.0
        };
        let total_content_width = total_child_width + total_gap_width;
        let free_space = container_width - total_content_width;

        match justify_content {
            JustifyContent::FlexStart => {
                let mut current_x = container_x;
                for (index, child) in children.iter().enumerate() {
                    let child_bounds = child.borrow().layout.bounds;
                    let child_style = child.borrow().layout.style.clone();
                    let (margin_top, margin_right, margin_bottom, margin_left) =
                        self.calculate_margin(&child_style);

                    // Position child on main axis with left margin
                    let mut child_borrow = child.borrow_mut();
                    child_borrow.layout.bounds.x = current_x + margin_left;

                    // Apply cross-axis alignment with top margin
                    self.apply_align_items_row_with_margin(
                        &mut child_borrow,
                        container_y,
                        container_height,
                        align_items,
                        margin_top,
                        margin_bottom,
                    );

                    current_x += child_bounds.width + margin_left + margin_right;

                    // Add gap after each item except the last
                    if index < children.len() - 1 {
                        current_x += column_gap;
                    }
                }
            }
            JustifyContent::FlexEnd => {
                let mut current_x = container_x + free_space;
                for (index, child) in children.iter().enumerate() {
                    let child_bounds = child.borrow().layout.bounds;

                    // Position child on main axis
                    let mut child_borrow = child.borrow_mut();
                    child_borrow.layout.bounds.x = current_x;

                    // Apply cross-axis alignment
                    self.apply_align_items_row(
                        &mut child_borrow,
                        container_y,
                        container_height,
                        align_items,
                    );

                    current_x += child_bounds.width;

                    // Add gap after each item except the last
                    if index < children.len() - 1 {
                        current_x += column_gap;
                    }
                }
            }
            JustifyContent::Center => {
                let mut current_x = container_x + free_space / 2.0;
                for (index, child) in children.iter().enumerate() {
                    let child_bounds = child.borrow().layout.bounds;

                    // Position child on main axis
                    let mut child_borrow = child.borrow_mut();
                    child_borrow.layout.bounds.x = current_x;

                    // Apply cross-axis alignment
                    self.apply_align_items_row(
                        &mut child_borrow,
                        container_y,
                        container_height,
                        align_items,
                    );

                    current_x += child_bounds.width;

                    // Add gap after each item except the last
                    if index < children.len() - 1 {
                        current_x += column_gap;
                    }
                }
            }
            JustifyContent::SpaceBetween => {
                if children.len() <= 1 {
                    // If only one child, behave like flex-start
                    self.apply_justify_content_row_single(
                        children,
                        container_x,
                        container_y,
                        container_height,
                        align_items,
                    );
                } else {
                    // Distribute free space evenly between items (in addition to gaps)
                    let extra_gap = free_space / (children.len() - 1) as f64;
                    let mut current_x = container_x;

                    for (index, child) in children.iter().enumerate() {
                        let child_bounds = child.borrow().layout.bounds;

                        // Position child on main axis
                        let mut child_borrow = child.borrow_mut();
                        child_borrow.layout.bounds.x = current_x;

                        // Apply cross-axis alignment
                        self.apply_align_items_row(
                            &mut child_borrow,
                            container_y,
                            container_height,
                            align_items,
                        );

                        current_x += child_bounds.width;

                        // Add gap and extra space after each item except the last
                        if index < children.len() - 1 {
                            current_x += column_gap + extra_gap;
                        }
                    }
                }
            }
            JustifyContent::SpaceAround => {
                let extra_gap = free_space / children.len() as f64;
                let mut current_x = container_x + extra_gap / 2.0;

                for (index, child) in children.iter().enumerate() {
                    let child_bounds = child.borrow().layout.bounds;

                    // Position child on main axis
                    let mut child_borrow = child.borrow_mut();
                    child_borrow.layout.bounds.x = current_x;

                    // Apply cross-axis alignment
                    self.apply_align_items_row(
                        &mut child_borrow,
                        container_y,
                        container_height,
                        align_items,
                    );

                    current_x += child_bounds.width;

                    // Add gap and extra space after each item
                    if index < children.len() - 1 {
                        current_x += column_gap + extra_gap;
                    }
                }
            }
            JustifyContent::SpaceEvenly => {
                let extra_gap = free_space / (children.len() + 1) as f64;
                let mut current_x = container_x + extra_gap;

                for (index, child) in children.iter().enumerate() {
                    let child_bounds = child.borrow().layout.bounds;

                    // Position child on main axis
                    let mut child_borrow = child.borrow_mut();
                    child_borrow.layout.bounds.x = current_x;

                    // Apply cross-axis alignment
                    self.apply_align_items_row(
                        &mut child_borrow,
                        container_y,
                        container_height,
                        align_items,
                    );

                    current_x += child_bounds.width;

                    // Add gap and extra space after each item except the last
                    if index < children.len() - 1 {
                        current_x += column_gap + extra_gap;
                    }
                }
            }
        }
    }

    /// Apply justify-content for single child (helper method)
    fn apply_justify_content_row_single(
        &self,
        children: &[Rc<RefCell<Node>>],
        container_x: f64,
        container_y: f64,
        container_height: f64,
        align_items: &AlignItems,
    ) {
        for child in children {
            let mut child_borrow = child.borrow_mut();
            child_borrow.layout.bounds.x = container_x;
            self.apply_align_items_row(
                &mut child_borrow,
                container_y,
                container_height,
                align_items,
            );
        }
    }

    /// Apply align-items positioning for row direction
    fn apply_align_items_row(
        &self,
        child: &mut std::cell::RefMut<Node>,
        container_y: f64,
        container_height: f64,
        align_items: &AlignItems,
    ) {
        match align_items {
            AlignItems::FlexStart => {
                child.layout.bounds.y = container_y;
            }
            AlignItems::Center => {
                // Center the child vertically within the container
                let child_height = child.layout.bounds.height;
                child.layout.bounds.y = container_y + (container_height - child_height) / 2.0;
            }
            AlignItems::FlexEnd => {
                // Align child to the bottom of the container
                let child_height = child.layout.bounds.height;
                child.layout.bounds.y = container_y + container_height - child_height;
            }
            AlignItems::Stretch => {
                // Stretch child to fill container height
                child.layout.bounds.y = container_y;
                child.layout.bounds.height = container_height;
            }
            AlignItems::Baseline => {
                // TODO: Implement baseline alignment
                // For now, behave like flex-start
                child.layout.bounds.y = container_y;
            }
        }
    }

    /// Apply align-items positioning for row direction with margin support
    fn apply_align_items_row_with_margin(
        &self,
        child: &mut std::cell::RefMut<Node>,
        container_y: f64,
        container_height: f64,
        align_items: &AlignItems,
        margin_top: f64,
        margin_bottom: f64,
    ) {
        match align_items {
            AlignItems::FlexStart => {
                child.layout.bounds.y = container_y + margin_top;
            }
            AlignItems::Center => {
                // Center the child vertically within the container, accounting for margins
                let child_height = child.layout.bounds.height;
                let available_height = container_height - margin_top - margin_bottom;
                child.layout.bounds.y =
                    container_y + margin_top + (available_height - child_height) / 2.0;
            }
            AlignItems::FlexEnd => {
                // Align child to the bottom of the container, accounting for bottom margin
                let child_height = child.layout.bounds.height;
                child.layout.bounds.y =
                    container_y + container_height - child_height - margin_bottom;
            }
            AlignItems::Stretch => {
                // Stretch child to fill container height minus margins
                child.layout.bounds.y = container_y + margin_top;
                child.layout.bounds.height = container_height - margin_top - margin_bottom;
            }
            AlignItems::Baseline => {
                // TODO: Implement baseline alignment with margins
                // For now, behave like flex-start
                child.layout.bounds.y = container_y + margin_top;
            }
        }
    }

    /// Calculate the starting Y positions for each line based on align-content with gap support
    fn calculate_line_positions_with_gap(
        &self,
        line_heights: &[f64],
        container_y: f64,
        container_height: f64,
        align_content: &AlignContent,
        row_gap: f64,
    ) -> Vec<f64> {
        let total_lines_height: f64 = line_heights.iter().sum();
        let total_gap_height = if line_heights.len() > 1 {
            row_gap * (line_heights.len() - 1) as f64
        } else {
            0.0
        };
        let total_content_height = total_lines_height + total_gap_height;
        let free_space = container_height - total_content_height;
        let mut positions = Vec::new();

        match align_content {
            AlignContent::FlexStart => {
                let mut current_y = container_y;
                for (index, &line_height) in line_heights.iter().enumerate() {
                    positions.push(current_y);
                    current_y += line_height;

                    // Add row gap after each line except the last
                    if index < line_heights.len() - 1 {
                        current_y += row_gap;
                    }
                }
            }
            AlignContent::FlexEnd => {
                let mut current_y = container_y + free_space;
                for (index, &line_height) in line_heights.iter().enumerate() {
                    positions.push(current_y);
                    current_y += line_height;

                    // Add row gap after each line except the last
                    if index < line_heights.len() - 1 {
                        current_y += row_gap;
                    }
                }
            }
            AlignContent::Center => {
                let mut current_y = container_y + free_space / 2.0;
                for (index, &line_height) in line_heights.iter().enumerate() {
                    positions.push(current_y);
                    current_y += line_height;

                    // Add row gap after each line except the last
                    if index < line_heights.len() - 1 {
                        current_y += row_gap;
                    }
                }
            }
            AlignContent::SpaceBetween => {
                if line_heights.len() <= 1 {
                    // If only one line, behave like flex-start
                    positions.push(container_y);
                } else {
                    let extra_gap = free_space / (line_heights.len() - 1) as f64;
                    let mut current_y = container_y;
                    for (index, &line_height) in line_heights.iter().enumerate() {
                        positions.push(current_y);
                        current_y += line_height;

                        // Add row gap and extra space after each line except the last
                        if index < line_heights.len() - 1 {
                            current_y += row_gap + extra_gap;
                        }
                    }
                }
            }
            AlignContent::SpaceAround => {
                let extra_gap = free_space / line_heights.len() as f64;
                let mut current_y = container_y + extra_gap / 2.0;
                for (index, &line_height) in line_heights.iter().enumerate() {
                    positions.push(current_y);
                    current_y += line_height;

                    // Add row gap and extra space after each line except the last
                    if index < line_heights.len() - 1 {
                        current_y += row_gap + extra_gap;
                    }
                }
            }
            AlignContent::SpaceEvenly => {
                let extra_gap = free_space / (line_heights.len() + 1) as f64;
                let mut current_y = container_y + extra_gap;
                for (index, &line_height) in line_heights.iter().enumerate() {
                    positions.push(current_y);
                    current_y += line_height;

                    // Add row gap and extra space after each line except the last
                    if index < line_heights.len() - 1 {
                        current_y += row_gap + extra_gap;
                    }
                }
            }
            AlignContent::Stretch => {
                // Stretch lines to fill the container
                if line_heights.is_empty() {
                    return positions;
                }

                let stretched_line_height = container_height / line_heights.len() as f64;
                let mut current_y = container_y;
                for _ in line_heights {
                    positions.push(current_y);
                    current_y += stretched_line_height;
                }
            }
        }

        positions
    }
}

impl Default for FlexLayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}
