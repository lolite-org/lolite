use super::*;
use std::rc::Rc;

// Helper function to create a basic engine setup
fn create_test_engine() -> Engine {
    Engine::new()
}

// Helper function to create a container with specified flex alignment properties
fn create_flex_container_with_alignment(
    engine: &mut Engine,
    flex_direction: Option<FlexDirection>,
    justify_content: Option<JustifyContent>,
    align_items: Option<AlignItems>,
    align_content: Option<AlignContent>,
    width: Option<f64>,
    height: Option<f64>,
) -> Id {
    let container_id = engine.document.create_node_autoid(None);

    // Add a CSS rule for the flex container
    let class_name = format!("flex_container_{}", container_id.0);
    engine.style_sheet.add_rule(Rule {
        selector: Selector::Class(class_name.clone()),
        declarations: vec![Style {
            display: Display::Flex,
            flex_direction,
            justify_content,
            align_items,
            align_content,
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
    let item_id = engine.document.create_node_autoid(Some("item".to_string()));

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

// JUSTIFY-CONTENT TESTS

#[test]
fn test_justify_content_flex_start() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a flex container with justify-content: flex-start
    let container = create_flex_container_with_alignment(
        &mut engine,
        Some(FlexDirection::Row),
        Some(JustifyContent::FlexStart),
        Some(AlignItems::FlexStart), // Preserve original dimensions
        None,
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

    // In flex-start, items should be at the start of the main axis
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);

    // Verify dimensions are preserved
    assert_eq!(w1, 50.0);
    assert_eq!(h1, 30.0);
    assert_eq!(w2, 60.0);
    assert_eq!(h2, 40.0);

    // Items should be positioned at the start (default behavior)
    assert_eq!(x1, 0.0); // First item at container start
    assert_eq!(x2, 50.0); // Second item after first
    assert_eq!(y1, 0.0); // Default cross-axis position
    assert_eq!(y2, 0.0); // Same cross-axis position
}

#[test]
fn test_justify_content_flex_end() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a flex container with justify-content: flex-end
    let container = create_flex_container_with_alignment(
        &mut engine,
        Some(FlexDirection::Row),
        Some(JustifyContent::FlexEnd),
        Some(AlignItems::FlexStart), // Preserve original dimensions
        None,
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

    // In flex-end, items should be at the end of the main axis
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);

    // Verify dimensions are preserved
    assert_eq!(w1, 50.0);
    assert_eq!(h1, 30.0);
    assert_eq!(w2, 60.0);
    assert_eq!(h2, 40.0);

    // Items should be positioned at the end
    // Total item width: 50 + 60 = 110
    // Container width: 300
    // Start position: 300 - 110 = 190
    assert_eq!(x1, 190.0); // First item at end position
    assert_eq!(x2, 240.0); // Second item after first (190 + 50)
    assert_eq!(y1, 0.0); // Default cross-axis position
    assert_eq!(y2, 0.0); // Same cross-axis position
}

#[test]
fn test_justify_content_center() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a flex container with justify-content: center
    let container = create_flex_container_with_alignment(
        &mut engine,
        Some(FlexDirection::Row),
        Some(JustifyContent::Center),
        Some(AlignItems::FlexStart), // Preserve original dimensions
        None,
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

    // In center, items should be centered on the main axis
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);

    // Verify dimensions are preserved
    assert_eq!(w1, 50.0);
    assert_eq!(h1, 30.0);
    assert_eq!(w2, 60.0);
    assert_eq!(h2, 40.0);

    // Items should be centered
    // Total item width: 50 + 60 = 110
    // Container width: 300
    // Free space: 300 - 110 = 190
    // Start position: 190 / 2 = 95
    assert_eq!(x1, 95.0); // First item at center position
    assert_eq!(x2, 145.0); // Second item after first (95 + 50)
    assert_eq!(y1, 0.0); // Default cross-axis position
    assert_eq!(y2, 0.0); // Same cross-axis position
}

#[test]
fn test_justify_content_space_between() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a flex container with justify-content: space-between
    let container = create_flex_container_with_alignment(
        &mut engine,
        Some(FlexDirection::Row),
        Some(JustifyContent::SpaceBetween),
        Some(AlignItems::FlexStart), // Preserve original dimensions
        None,
        Some(300.0),
        Some(100.0),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create three flex items for better space-between testing
    let item1 = create_flex_item(&mut engine, 50.0, 30.0);
    let item2 = create_flex_item(&mut engine, 60.0, 40.0);
    let item3 = create_flex_item(&mut engine, 40.0, 35.0);

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();
    engine.document.set_parent(container, item3).unwrap();

    // Run layout
    engine.layout();

    // In space-between, items should be distributed with equal space between them
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);
    let (x3, y3, w3, h3) = get_bounds(&engine, item3);

    // Verify dimensions are preserved
    assert_eq!(w1, 50.0);
    assert_eq!(h1, 30.0);
    assert_eq!(w2, 60.0);
    assert_eq!(h2, 40.0);
    assert_eq!(w3, 40.0);
    assert_eq!(h3, 35.0);

    // Items should be distributed with equal space between
    // Total item width: 50 + 60 + 40 = 150
    // Container width: 300
    // Free space: 300 - 150 = 150
    // Space between items: 150 / 2 = 75 (for 3 items, 2 gaps)
    assert_eq!(x1, 0.0); // First item at start
    assert_eq!(x2, 125.0); // Second item (0 + 50 + 75)
    assert_eq!(x3, 260.0); // Third item (125 + 60 + 75)

    // All items on same cross-axis position
    assert_eq!(y1, 0.0);
    assert_eq!(y2, 0.0);
    assert_eq!(y3, 0.0);
}

// ALIGN-ITEMS TESTS

#[test]
fn test_align_items_flex_start() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a flex container with align-items: flex-start
    let container = create_flex_container_with_alignment(
        &mut engine,
        Some(FlexDirection::Row),
        None,
        Some(AlignItems::FlexStart),
        None,
        Some(300.0),
        Some(100.0),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create items with different heights
    let item1 = create_flex_item(&mut engine, 50.0, 30.0);
    let item2 = create_flex_item(&mut engine, 60.0, 50.0);

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();

    // Run layout
    engine.layout();

    // In align-items: flex-start, items should align to the start of cross axis
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);

    // Verify dimensions are preserved
    assert_eq!(w1, 50.0);
    assert_eq!(h1, 30.0);
    assert_eq!(w2, 60.0);
    assert_eq!(h2, 50.0);

    // Main axis positioning (default flex-start)
    assert_eq!(x1, 0.0);
    assert_eq!(x2, 50.0);

    // Cross axis positioning (both should be at start)
    assert_eq!(y1, 0.0); // Aligned to top
    assert_eq!(y2, 0.0); // Aligned to top
}

#[test]
fn test_align_items_center() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a flex container with align-items: center
    let container = create_flex_container_with_alignment(
        &mut engine,
        Some(FlexDirection::Row),
        None,
        Some(AlignItems::Center),
        None,
        Some(300.0),
        Some(100.0),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create items with different heights
    let item1 = create_flex_item(&mut engine, 50.0, 30.0);
    let item2 = create_flex_item(&mut engine, 60.0, 50.0);

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();

    // Run layout
    engine.layout();

    // In align-items: center, items should be centered on cross axis
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);

    // Verify dimensions are preserved
    assert_eq!(w1, 50.0);
    assert_eq!(h1, 30.0);
    assert_eq!(w2, 60.0);
    assert_eq!(h2, 50.0);

    // Main axis positioning (default flex-start)
    assert_eq!(x1, 0.0);
    assert_eq!(x2, 50.0);

    // Cross axis positioning (centered in 100px container)
    assert_eq!(y1, 35.0); // (100 - 30) / 2 = 35
    assert_eq!(y2, 25.0); // (100 - 50) / 2 = 25
}

// ALIGN-CONTENT TESTS (for multi-line flex containers)

#[test]
fn test_align_content_flex_start() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a flex container with wrap and align-content: flex-start
    let container = create_flex_container_with_alignment(
        &mut engine,
        Some(FlexDirection::Row),
        None,
        None,
        Some(AlignContent::FlexStart),
        Some(200.0), // Small width to force wrapping
        Some(150.0),
    );
    engine.document.set_parent(root, container).unwrap();

    // Add wrapping to the container
    let container_node = engine.document.nodes.get(&container).unwrap();
    let mut style = container_node.borrow().layout.style.as_ref().clone();
    style.flex_wrap = Some(FlexWrap::Wrap);
    container_node.borrow_mut().layout.style = Rc::new(style);

    // Create items that will wrap to multiple lines
    let item1 = create_flex_item(&mut engine, 100.0, 30.0);
    let item2 = create_flex_item(&mut engine, 100.0, 30.0);
    let item3 = create_flex_item(&mut engine, 100.0, 30.0);

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();
    engine.document.set_parent(container, item3).unwrap();

    // Run layout
    engine.layout();

    // In align-content: flex-start, lines should start at the beginning of cross axis
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);
    let (x3, y3, w3, h3) = get_bounds(&engine, item3);

    // Verify dimensions are preserved
    assert_eq!(w1, 100.0);
    assert_eq!(h1, 30.0);
    assert_eq!(w2, 100.0);
    assert_eq!(h2, 30.0);
    assert_eq!(w3, 100.0);
    assert_eq!(h3, 30.0);

    // First line should be at top, second line below it
    assert_eq!(y1, 0.0); // First line at top
    assert_eq!(y2, 0.0); // Same line as item1
    assert_eq!(y3, 30.0); // Second line (wrapped)

    // Main axis positioning
    assert_eq!(x1, 0.0); // First item
    assert_eq!(x2, 100.0); // Second item on same line
    assert_eq!(x3, 0.0); // Third item wrapped to new line
}

// Basic setup test to ensure alignment containers work
#[test]
fn test_basic_alignment_setup() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a container with all alignment properties
    let container = create_flex_container_with_alignment(
        &mut engine,
        Some(FlexDirection::Row),
        Some(JustifyContent::FlexStart),
        Some(AlignItems::FlexStart),
        Some(AlignContent::FlexStart),
        Some(200.0),
        Some(100.0),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create one flex item
    let item = create_flex_item(&mut engine, 50.0, 30.0);
    engine.document.set_parent(container, item).unwrap();

    // Run layout
    engine.layout();

    // Verify the layout runs without errors
    let (x, y, w, h) = get_bounds(&engine, item);

    // Basic assertions
    assert_eq!(w, 50.0);
    assert_eq!(h, 30.0);
    assert_eq!(x, 0.0); // flex-start behavior
    assert_eq!(y, 0.0); // flex-start behavior
}
