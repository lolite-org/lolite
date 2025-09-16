use crate::style::{Display, FlexDirection, FlexWrap, JustifyContent, Length, Rule};

use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

fn next_test_id() -> Id {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    Id::from_u64(NEXT.fetch_add(1, Ordering::Relaxed))
}

// Helper function to create a basic engine setup
fn create_test_engine() -> Engine {
    Engine::new()
}

// Helper function to create a container with specified gap properties
fn create_flex_container_with_gap(
    engine: &mut Engine,
    flex_direction: Option<FlexDirection>,
    gap: Option<f64>,
    row_gap: Option<f64>,
    column_gap: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
) -> Id {
    let container_id = engine.document.create_node(next_test_id(), None);

    // Add a CSS rule for the flex container
    let class_name = format!("flex_container_{}", container_id.0);
    engine.style_sheet.add_rule(Rule {
        selector: Selector::Class(class_name.clone()),
        declarations: vec![Style {
            display: Display::Flex,
            flex_direction,
            gap: gap.map(Length::Px),
            row_gap: row_gap.map(Length::Px),
            column_gap: column_gap.map(Length::Px),
            width: width.map(Length::Px),
            height: height.map(Length::Px),
            ..Default::default()
        }],
    });

    engine
        .document
        .set_attribute(container_id, "class".to_owned(), class_name);
    container_id
}

// Helper function to create a flex item with specified dimensions
fn create_flex_item(engine: &mut Engine, width: f64, height: f64) -> Id {
    let item_id = engine
        .document
        .create_node(next_test_id(), Some("item".to_string()));

    // Add a CSS rule for the flex item
    let class_name = format!("flex_item_{}", item_id.0);
    engine.style_sheet.add_rule(Rule {
        selector: Selector::Class(class_name.clone()),
        declarations: vec![Style {
            width: Some(Length::Px(width)),
            height: Some(Length::Px(height)),
            ..Default::default()
        }],
    });

    engine
        .document
        .set_attribute(item_id, "class".to_owned(), class_name);
    item_id
}

// Helper function to get node bounds after layout
fn get_bounds(engine: &Engine, node_id: Id) -> (f64, f64, f64, f64) {
    let node = engine.document.nodes.get(&node_id).unwrap();
    let bounds = &node.borrow().layout.bounds;
    (bounds.x, bounds.y, bounds.width, bounds.height)
}

// COLUMN GAP TESTS

#[test]
fn test_column_gap_row_direction() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a flex container with column gap
    let container = create_flex_container_with_gap(
        &mut engine,
        Some(FlexDirection::Row),
        None,
        None,
        Some(20.0), // 20px column gap
        Some(300.0),
        Some(100.0),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create three flex items
    let item1 = create_flex_item(&mut engine, 50.0, 30.0);
    let item2 = create_flex_item(&mut engine, 60.0, 40.0);
    let item3 = create_flex_item(&mut engine, 70.0, 35.0);

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();
    engine.document.set_parent(container, item3).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning with column gaps
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);
    let (x3, y3, w3, h3) = get_bounds(&engine, item3);

    // Verify dimensions are preserved
    assert_eq!(w1, 50.0);
    assert_eq!(h1, 30.0);
    assert_eq!(w2, 60.0);
    assert_eq!(h2, 40.0);
    assert_eq!(w3, 70.0);
    assert_eq!(h3, 35.0);

    // Verify horizontal positioning with 20px gaps
    assert_eq!(x1, 0.0); // First item at container start
    assert_eq!(x2, 70.0); // Second item after first + gap (0 + 50 + 20)
    assert_eq!(x3, 150.0); // Third item after second + gap (70 + 60 + 20)

    // All items should be on the same horizontal line
    assert_eq!(y1, 0.0);
    assert_eq!(y2, 0.0);
    assert_eq!(y3, 0.0);
}

#[test]
fn test_row_gap_column_direction() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a flex container with row gap
    let container = create_flex_container_with_gap(
        &mut engine,
        Some(FlexDirection::Column),
        None,
        Some(15.0), // 15px row gap
        None,
        Some(100.0),
        Some(300.0),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create three flex items
    let item1 = create_flex_item(&mut engine, 50.0, 30.0);
    let item2 = create_flex_item(&mut engine, 60.0, 40.0);
    let item3 = create_flex_item(&mut engine, 70.0, 35.0);

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();
    engine.document.set_parent(container, item3).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning with row gaps
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);
    let (x3, y3, w3, h3) = get_bounds(&engine, item3);

    // Verify dimensions are preserved
    assert_eq!(w1, 50.0);
    assert_eq!(h1, 30.0);
    assert_eq!(w2, 60.0);
    assert_eq!(h2, 40.0);
    assert_eq!(w3, 70.0);
    assert_eq!(h3, 35.0);

    // Verify vertical positioning with 15px gaps
    assert_eq!(y1, 0.0); // First item at container start
    assert_eq!(y2, 45.0); // Second item after first + gap (0 + 30 + 15)
    assert_eq!(y3, 100.0); // Third item after second + gap (45 + 40 + 15)

    // All items should be aligned on the same vertical line
    assert_eq!(x1, 0.0);
    assert_eq!(x2, 0.0);
    assert_eq!(x3, 0.0);
}

#[test]
fn test_gap_shorthand_row_direction() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a flex container with gap shorthand (applies to both row and column gaps)
    let container = create_flex_container_with_gap(
        &mut engine,
        Some(FlexDirection::Row),
        Some(25.0), // 25px gap (both row and column)
        None,
        None,
        Some(300.0),
        Some(100.0),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create three flex items
    let item1 = create_flex_item(&mut engine, 50.0, 30.0);
    let item2 = create_flex_item(&mut engine, 60.0, 40.0);
    let item3 = create_flex_item(&mut engine, 70.0, 35.0);

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();
    engine.document.set_parent(container, item3).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning with gaps from shorthand
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);
    let (x3, y3, w3, h3) = get_bounds(&engine, item3);

    // Verify dimensions are preserved
    assert_eq!(w1, 50.0);
    assert_eq!(h1, 30.0);
    assert_eq!(w2, 60.0);
    assert_eq!(h2, 40.0);
    assert_eq!(w3, 70.0);
    assert_eq!(h3, 35.0);

    // Verify horizontal positioning with 25px gaps (gap shorthand acts as column-gap in row direction)
    assert_eq!(x1, 0.0); // First item at container start
    assert_eq!(x2, 75.0); // Second item after first + gap (0 + 50 + 25)
    assert_eq!(x3, 160.0); // Third item after second + gap (75 + 60 + 25)

    // All items should be on the same horizontal line
    assert_eq!(y1, 0.0);
    assert_eq!(y2, 0.0);
    assert_eq!(y3, 0.0);
}

#[test]
fn test_gap_priority_over_individual_gaps() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a flex container with both gap shorthand and individual gaps
    // The gap shorthand should take precedence
    let container = create_flex_container_with_gap(
        &mut engine,
        Some(FlexDirection::Row),
        Some(30.0), // 30px gap shorthand (should take precedence)
        Some(10.0), // 10px row gap (should be ignored)
        Some(15.0), // 15px column gap (should be ignored)
        Some(300.0),
        Some(100.0),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create two flex items
    let item1 = create_flex_item(&mut engine, 50.0, 30.0);
    let item2 = create_flex_item(&mut engine, 60.0, 40.0);

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning uses gap shorthand value (30px), not individual gaps
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);

    // Verify dimensions are preserved
    assert_eq!(w1, 50.0);
    assert_eq!(h1, 30.0);
    assert_eq!(w2, 60.0);
    assert_eq!(h2, 40.0);

    // Verify horizontal positioning with 30px gap (from shorthand, not 15px column-gap)
    assert_eq!(x1, 0.0); // First item at container start
    assert_eq!(x2, 80.0); // Second item after first + gap (0 + 50 + 30)

    // All items should be on the same horizontal line
    assert_eq!(y1, 0.0);
    assert_eq!(y2, 0.0);
}

// WRAPPED LAYOUT WITH GAPS TESTS

#[test]
fn test_gap_with_wrapped_layout() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a flex container with wrap and gaps
    let container = create_flex_container_with_gap(
        &mut engine,
        Some(FlexDirection::Row),
        None,
        Some(20.0),  // 20px row gap
        Some(10.0),  // 10px column gap
        Some(150.0), // Small width to force wrapping
        Some(200.0),
    );
    engine.document.set_parent(root, container).unwrap();

    // Add wrapping to the container
    let container_node = engine.document.nodes.get(&container).unwrap();
    let mut style = container_node.borrow().layout.style.as_ref().clone();
    style.flex_wrap = Some(FlexWrap::Wrap);
    container_node.borrow_mut().layout.style = Arc::new(style);

    // Create items that will wrap to multiple lines
    let item1 = create_flex_item(&mut engine, 60.0, 30.0);
    let item2 = create_flex_item(&mut engine, 60.0, 30.0);
    let item3 = create_flex_item(&mut engine, 60.0, 30.0);

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();
    engine.document.set_parent(container, item3).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning with gaps in wrapped layout
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);
    let (x3, y3, w3, h3) = get_bounds(&engine, item3);

    // Verify dimensions are preserved
    assert_eq!(w1, 60.0);
    assert_eq!(h1, 30.0);
    assert_eq!(w2, 60.0);
    assert_eq!(h2, 30.0);
    assert_eq!(w3, 60.0);
    assert_eq!(h3, 30.0);

    // First line: item1 and item2 with column gap
    assert_eq!(x1, 0.0); // First item at container start
    assert_eq!(x2, 70.0); // Second item after first + column gap (0 + 60 + 10)
    assert_eq!(y1, 0.0); // First line
    assert_eq!(y2, 0.0); // Same line as item1

    // Third item should wrap to second line with row gap
    assert_eq!(x3, 0.0); // Wrapped item starts at container left
    assert_eq!(y3, 50.0); // Second line with row gap (0 + 30 + 20)

    // Verify that third item is on a different line
    assert!(y3 > y1, "Item 3 should be on a different line than item 1");
}

#[test]
fn test_no_gap_default_behavior() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a flex container without any gap properties
    let container = create_flex_container_with_gap(
        &mut engine,
        Some(FlexDirection::Row),
        None, // No gap
        None, // No row gap
        None, // No column gap
        Some(300.0),
        Some(100.0),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create three flex items
    let item1 = create_flex_item(&mut engine, 50.0, 30.0);
    let item2 = create_flex_item(&mut engine, 60.0, 40.0);
    let item3 = create_flex_item(&mut engine, 70.0, 35.0);

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();
    engine.document.set_parent(container, item3).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning without gaps (should be same as before gap implementation)
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);
    let (x3, y3, w3, h3) = get_bounds(&engine, item3);

    // Verify dimensions are preserved
    assert_eq!(w1, 50.0);
    assert_eq!(h1, 30.0);
    assert_eq!(w2, 60.0);
    assert_eq!(h2, 40.0);
    assert_eq!(w3, 70.0);
    assert_eq!(h3, 35.0);

    // Verify horizontal positioning without gaps
    assert_eq!(x1, 0.0); // First item at container start
    assert_eq!(x2, 50.0); // Second item after first (0 + 50)
    assert_eq!(x3, 110.0); // Third item after second (50 + 60)

    // All items should be on the same horizontal line
    assert_eq!(y1, 0.0);
    assert_eq!(y2, 0.0);
    assert_eq!(y3, 0.0);
}

#[test]
fn test_gap_with_justify_content_center() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a flex container with gap and justify-content: center
    let container_id = engine.document.create_node(next_test_id(), None);
    let class_name = format!("flex_container_{}", container_id.0);
    engine.style_sheet.add_rule(Rule {
        selector: Selector::Class(class_name.clone()),
        declarations: vec![Style {
            display: Display::Flex,
            flex_direction: Some(FlexDirection::Row),
            justify_content: Some(JustifyContent::Center),
            column_gap: Some(Length::Px(20.0)),
            width: Some(Length::Px(300.0)),
            height: Some(Length::Px(100.0)),
            ..Default::default()
        }],
    });
    engine
        .document
        .set_attribute(container_id, "class".to_owned(), class_name);
    engine.document.set_parent(root, container_id).unwrap();

    // Create two flex items
    let item1 = create_flex_item(&mut engine, 50.0, 30.0);
    let item2 = create_flex_item(&mut engine, 60.0, 40.0);

    engine.document.set_parent(container_id, item1).unwrap();
    engine.document.set_parent(container_id, item2).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning with gap and center alignment
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);

    // Verify dimensions are preserved
    assert_eq!(w1, 50.0);
    assert_eq!(h1, 30.0);
    assert_eq!(w2, 60.0);
    assert_eq!(h2, 40.0);

    // Total content width: 50 + 60 + 20 (gap) = 130
    // Container width: 300
    // Free space: 300 - 130 = 170
    // Start position for centering: 170 / 2 = 85
    assert_eq!(x1, 85.0); // First item centered
    assert_eq!(x2, 155.0); // Second item after first + gap (85 + 50 + 20)

    // All items should be on the same horizontal line
    assert_eq!(y1, 0.0);
    assert_eq!(y2, 0.0);
}

#[test]
fn test_gap_with_justify_content_space_between() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a flex container with gap and justify-content: space-between
    let container_id = engine.document.create_node(next_test_id(), None);
    let class_name = format!("flex_container_{}", container_id.0);
    engine.style_sheet.add_rule(Rule {
        selector: Selector::Class(class_name.clone()),
        declarations: vec![Style {
            display: Display::Flex,
            flex_direction: Some(FlexDirection::Row),
            justify_content: Some(JustifyContent::SpaceBetween),
            column_gap: Some(Length::Px(10.0)),
            width: Some(Length::Px(300.0)),
            height: Some(Length::Px(100.0)),
            ..Default::default()
        }],
    });
    engine
        .document
        .set_attribute(container_id, "class".to_owned(), class_name);
    engine.document.set_parent(root, container_id).unwrap();

    // Create three flex items
    let item1 = create_flex_item(&mut engine, 40.0, 30.0);
    let item2 = create_flex_item(&mut engine, 50.0, 40.0);
    let item3 = create_flex_item(&mut engine, 60.0, 35.0);

    engine.document.set_parent(container_id, item1).unwrap();
    engine.document.set_parent(container_id, item2).unwrap();
    engine.document.set_parent(container_id, item3).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning with gap and space-between alignment
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);
    let (x3, y3, w3, h3) = get_bounds(&engine, item3);

    // Verify dimensions are preserved
    assert_eq!(w1, 40.0);
    assert_eq!(h1, 30.0);
    assert_eq!(w2, 50.0);
    assert_eq!(h2, 40.0);
    assert_eq!(w3, 60.0);
    assert_eq!(h3, 35.0);

    // Total content width: 40 + 50 + 60 + 20 (2 gaps) = 170
    // Container width: 300
    // Free space: 300 - 170 = 130
    // Extra space between items: 130 / 2 = 65 (for 3 items, 2 gaps)
    assert_eq!(x1, 0.0); // First item at start
    assert_eq!(x2, 115.0); // Second item (0 + 40 + 10 + 65)
    assert_eq!(x3, 240.0); // Third item (115 + 50 + 10 + 65)

    // All items should be on the same horizontal line
    assert_eq!(y1, 0.0);
    assert_eq!(y2, 0.0);
    assert_eq!(y3, 0.0);
}
