use super::*;

// Helper function to create a basic engine setup
fn create_test_engine() -> Engine {
    Engine::new()
}

// Helper function to create a flex container with specified properties
fn create_flex_container(
    engine: &mut Engine,
    flex_direction: Option<FlexDirection>,
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

// Helper function to create a flex item with flex properties
fn create_flex_item_with_flex(
    engine: &mut Engine,
    width: Option<f64>,
    height: Option<f64>,
    flex_grow: Option<f64>,
    flex_shrink: Option<f64>,
    flex_basis: Option<Length>,
) -> Id {
    let item_id = engine.document.create_node_autoid(Some("item".to_string()));

    // Add a CSS rule for the flex item
    let class_name = format!("flex_item_{}", item_id.0);
    engine.style_sheet.add_rule(Rule {
        selector: Selector::Class(class_name.clone()),
        declarations: vec![Style {
            width: width.map(Length::Px),
            height: height.map(Length::Px),
            flex_grow,
            flex_shrink,
            flex_basis,
            ..Default::default()
        }],
    });

    engine
        .document
        .set_attribute(item_id, "class".to_owned(), class_name);
    item_id
}

// Helper function to create a simple flex item
fn create_flex_item(engine: &mut Engine, width: f64, height: f64) -> Id {
    create_flex_item_with_flex(engine, Some(width), Some(height), None, None, None)
}

// Helper function to get node bounds after layout
fn get_bounds(engine: &Engine, node_id: Id) -> (f64, f64, f64, f64) {
    let node = engine.document.nodes.get(&node_id).unwrap();
    let bounds = &node.borrow().layout.bounds;
    (bounds.x, bounds.y, bounds.width, bounds.height)
}

// FLEX-GROW TESTS

#[test]
fn test_flex_grow_basic() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a 300px wide container
    let container = create_flex_container(
        &mut engine,
        Some(FlexDirection::Row),
        Some(300.0),
        Some(100.0),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create items: one with flex-grow: 1, one without
    let item1 =
        create_flex_item_with_flex(&mut engine, Some(50.0), Some(30.0), Some(1.0), None, None);
    let item2 = create_flex_item(&mut engine, 50.0, 30.0);

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning and sizing
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);

    // Item2 should keep its original width (50px)
    assert_eq!(w2, 50.0);
    assert_eq!(h2, 30.0);
    assert_eq!(x2, 250.0); // After item1 which should have grown

    // Item1 should grow to fill remaining space (300 - 100 = 200px available, 50px base + 200px growth)
    assert_eq!(w1, 250.0);
    assert_eq!(h1, 30.0);
    assert_eq!(x1, 0.0);

    // Both items should be on the same line
    assert_eq!(y1, 0.0);
    assert_eq!(y2, 0.0);
}

#[test]
fn test_flex_grow_multiple_items() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a 400px wide container
    let container = create_flex_container(
        &mut engine,
        Some(FlexDirection::Row),
        Some(400.0),
        Some(100.0),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create items with different flex-grow values
    let item1 =
        create_flex_item_with_flex(&mut engine, Some(50.0), Some(30.0), Some(1.0), None, None);
    let item2 =
        create_flex_item_with_flex(&mut engine, Some(50.0), Some(30.0), Some(2.0), None, None);
    let item3 = create_flex_item(&mut engine, 50.0, 30.0); // No flex-grow

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();
    engine.document.set_parent(container, item3).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning and sizing
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);
    let (x3, y3, w3, h3) = get_bounds(&engine, item3);

    // Item3 should keep its original width (50px)
    assert_eq!(w3, 50.0);
    assert_eq!(h3, 30.0);

    // Available space: 400 - 150 = 250px
    // Flex-grow ratio: 1:2 (total = 3)
    // Item1 gets 250 * (1/3) = ~83.33px extra -> 50 + 83.33 = ~133.33px
    // Item2 gets 250 * (2/3) = ~166.67px extra -> 50 + 166.67 = ~216.67px
    assert!((w1 - 133.33).abs() < 0.1);
    assert!((w2 - 216.67).abs() < 0.1);

    // Verify positioning
    assert_eq!(x1, 0.0);
    assert!((x2 - w1).abs() < 0.1);
    assert!((x3 - w1 - w2).abs() < 0.1);

    // All items should be on the same line
    assert_eq!(y1, 0.0);
    assert_eq!(y2, 0.0);
    assert_eq!(y3, 0.0);
}

#[test]
fn test_flex_grow_column_direction() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a 400px high container in column direction
    let container = create_flex_container(
        &mut engine,
        Some(FlexDirection::Column),
        Some(100.0),
        Some(400.0),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create items with flex-grow in column direction
    let item1 =
        create_flex_item_with_flex(&mut engine, Some(50.0), Some(50.0), Some(1.0), None, None);
    let item2 = create_flex_item(&mut engine, 50.0, 50.0); // No flex-grow

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning and sizing
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);

    // Item2 should keep its original height (50px)
    assert_eq!(w2, 50.0);
    assert_eq!(h2, 50.0);
    assert_eq!(y2, 350.0); // After item1 which should have grown

    // Item1 should grow to fill remaining space (400 - 100 = 300px available, 50px base + 300px growth)
    assert_eq!(w1, 50.0);
    assert_eq!(h1, 350.0);
    assert_eq!(y1, 0.0);

    // Both items should be on the same column
    assert_eq!(x1, 0.0);
    assert_eq!(x2, 0.0);
}

// FLEX-SHRINK TESTS

#[test]
fn test_flex_shrink_basic() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a 200px wide container (smaller than total item width)
    let container = create_flex_container(
        &mut engine,
        Some(FlexDirection::Row),
        Some(200.0),
        Some(100.0),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create items that would overflow: 150px + 150px = 300px > 200px container
    let item1 =
        create_flex_item_with_flex(&mut engine, Some(150.0), Some(30.0), None, Some(1.0), None);
    let item2 =
        create_flex_item_with_flex(&mut engine, Some(150.0), Some(30.0), None, Some(2.0), None);

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning and sizing
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);

    // Overflow: 300 - 200 = 100px needs to be shrunk
    // Flex-shrink ratio: 1:2 (total = 3)
    // Item1 shrinks by 100 * (1/3) = ~33.33px -> 150 - 33.33 = ~116.67px
    // Item2 shrinks by 100 * (2/3) = ~66.67px -> 150 - 66.67 = ~83.33px
    assert!((w1 - 116.67).abs() < 0.1);
    assert!((w2 - 83.33).abs() < 0.1);

    // Verify positioning
    assert_eq!(x1, 0.0);
    assert!((x2 - w1).abs() < 0.1);

    // Both items should be on the same line
    assert_eq!(y1, 0.0);
    assert_eq!(y2, 0.0);
}

#[test]
fn test_flex_shrink_column_direction() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a 200px high container (smaller than total item height)
    let container = create_flex_container(
        &mut engine,
        Some(FlexDirection::Column),
        Some(100.0),
        Some(200.0),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create items that would overflow: 150px + 100px = 250px > 200px container
    let item1 =
        create_flex_item_with_flex(&mut engine, Some(50.0), Some(150.0), None, Some(1.0), None);
    let item2 =
        create_flex_item_with_flex(&mut engine, Some(50.0), Some(100.0), None, Some(1.0), None);

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning and sizing
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);

    // Overflow: 250 - 200 = 50px needs to be shrunk
    // Equal flex-shrink ratios, so shrink proportionally to their base sizes
    // Item1: 150px base, shrinks by 50 * (150/250) = 30px -> 150 - 30 = 120px
    // Item2: 100px base, shrinks by 50 * (100/250) = 20px -> 100 - 20 = 80px
    assert_eq!(h1, 120.0);
    assert_eq!(h2, 80.0);

    // Verify positioning
    assert_eq!(y1, 0.0);
    assert_eq!(y2, 120.0);

    // Both items should be on the same column
    assert_eq!(x1, 0.0);
    assert_eq!(x2, 0.0);
}

// FLEX-BASIS TESTS

#[test]
fn test_flex_basis_auto() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a container
    let container = create_flex_container(
        &mut engine,
        Some(FlexDirection::Row),
        Some(300.0),
        Some(100.0),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create items with flex-basis: auto (should use width/height)
    let item1 = create_flex_item_with_flex(
        &mut engine,
        Some(50.0),
        Some(30.0),
        None,
        None,
        Some(Length::Auto),
    );
    let item2 = create_flex_item_with_flex(
        &mut engine,
        Some(100.0),
        Some(30.0),
        None,
        None,
        Some(Length::Auto),
    );

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning and sizing
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);

    // With flex-basis: auto, items should use their width values
    assert_eq!(w1, 50.0);
    assert_eq!(w2, 100.0);
    assert_eq!(h1, 30.0);
    assert_eq!(h2, 30.0);

    // Verify positioning
    assert_eq!(x1, 0.0);
    assert_eq!(x2, 50.0);

    // Both items should be on the same line
    assert_eq!(y1, 0.0);
    assert_eq!(y2, 0.0);
}

#[test]
fn test_flex_basis_fixed_size() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a container
    let container = create_flex_container(
        &mut engine,
        Some(FlexDirection::Row),
        Some(300.0),
        Some(100.0),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create items with specific flex-basis values (should override width)
    let item1 = create_flex_item_with_flex(
        &mut engine,
        Some(50.0),
        Some(30.0),
        None,
        None,
        Some(Length::Px(80.0)),
    );
    let item2 = create_flex_item_with_flex(
        &mut engine,
        Some(100.0),
        Some(30.0),
        None,
        None,
        Some(Length::Px(120.0)),
    );

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning and sizing
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);

    // With specific flex-basis values, items should use those instead of width
    assert_eq!(w1, 80.0);
    assert_eq!(w2, 120.0);
    assert_eq!(h1, 30.0); // Height should remain from original
    assert_eq!(h2, 30.0);

    // Verify positioning
    assert_eq!(x1, 0.0);
    assert_eq!(x2, 80.0);

    // Both items should be on the same line
    assert_eq!(y1, 0.0);
    assert_eq!(y2, 0.0);
}

#[test]
fn test_flex_basis_column_direction() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a container in column direction
    let container = create_flex_container(
        &mut engine,
        Some(FlexDirection::Column),
        Some(100.0),
        Some(300.0),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create items with flex-basis in column direction (should affect height)
    let item1 = create_flex_item_with_flex(
        &mut engine,
        Some(50.0),
        Some(60.0),
        None,
        None,
        Some(Length::Px(80.0)),
    );
    let item2 = create_flex_item_with_flex(
        &mut engine,
        Some(50.0),
        Some(40.0),
        None,
        None,
        Some(Length::Px(120.0)),
    );

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning and sizing
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);

    // In column direction, flex-basis should affect height
    assert_eq!(w1, 50.0); // Width should remain from original
    assert_eq!(w2, 50.0);
    assert_eq!(h1, 80.0); // Height should use flex-basis
    assert_eq!(h2, 120.0);

    // Verify positioning
    assert_eq!(y1, 0.0);
    assert_eq!(y2, 80.0);

    // Both items should be on the same column
    assert_eq!(x1, 0.0);
    assert_eq!(x2, 0.0);
}

// COMBINED TESTS

#[test]
fn test_flex_grow_shrink_basis_combined() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a 300px wide container
    let container = create_flex_container(
        &mut engine,
        Some(FlexDirection::Row),
        Some(300.0),
        Some(100.0),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create items with combined flex properties
    let item1 = create_flex_item_with_flex(
        &mut engine,
        Some(50.0), // width (ignored due to flex-basis)
        Some(30.0),
        Some(2.0),               // flex-grow
        Some(1.0),               // flex-shrink
        Some(Length::Px(100.0)), // flex-basis
    );
    let item2 = create_flex_item_with_flex(
        &mut engine,
        Some(60.0), // width (ignored due to flex-basis)
        Some(30.0),
        Some(1.0),              // flex-grow
        Some(1.0),              // flex-shrink
        Some(Length::Px(80.0)), // flex-basis
    );

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning and sizing
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);

    // Base sizes from flex-basis: 100px + 80px = 180px
    // Available space: 300px - 180px = 120px
    // Flex-grow ratio: 2:1 (total = 3)
    // Item1 gets 120 * (2/3) = 80px extra -> 100 + 80 = 180px
    // Item2 gets 120 * (1/3) = 40px extra -> 80 + 40 = 120px
    assert_eq!(w1, 180.0);
    assert_eq!(w2, 120.0);

    // Verify positioning
    assert_eq!(x1, 0.0);
    assert_eq!(x2, 180.0);

    // Both items should be on the same line
    assert_eq!(y1, 0.0);
    assert_eq!(y2, 0.0);
}

#[test]
fn test_flex_shrink_with_basis_overflow() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a 200px wide container (smaller than total flex-basis)
    let container = create_flex_container(
        &mut engine,
        Some(FlexDirection::Row),
        Some(200.0),
        Some(100.0),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create items with flex-basis that would overflow
    let item1 = create_flex_item_with_flex(
        &mut engine,
        Some(50.0), // width (ignored)
        Some(30.0),
        None,                    // no flex-grow
        Some(1.0),               // flex-shrink
        Some(Length::Px(150.0)), // flex-basis
    );
    let item2 = create_flex_item_with_flex(
        &mut engine,
        Some(60.0), // width (ignored)
        Some(30.0),
        None,                    // no flex-grow
        Some(2.0),               // flex-shrink (higher)
        Some(Length::Px(100.0)), // flex-basis
    );

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning and sizing
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);

    // Base sizes from flex-basis: 150px + 100px = 250px
    // Overflow: 250px - 200px = 50px needs to be shrunk
    // Weighted flex-shrink ratio: (1*150):(2*100) = 150:200 = 3:4 (total = 7)
    // Item1 shrinks by 50 * (150/350) = ~21.43px -> 150 - 21.43 = ~128.57px
    // Item2 shrinks by 50 * (200/350) = ~28.57px -> 100 - 28.57 = ~71.43px
    assert!((w1 - 128.57).abs() < 0.1);
    assert!((w2 - 71.43).abs() < 0.1);

    // Verify positioning
    assert_eq!(x1, 0.0);
    assert!((x2 - w1).abs() < 0.1);

    // Both items should be on the same line
    assert_eq!(y1, 0.0);
    assert_eq!(y2, 0.0);
}

#[test]
fn test_flex_zero_values() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a container
    let container = create_flex_container(
        &mut engine,
        Some(FlexDirection::Row),
        Some(300.0),
        Some(100.0),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create items with zero flex values
    let item1 = create_flex_item_with_flex(
        &mut engine,
        Some(100.0),
        Some(30.0),
        Some(0.0), // flex-grow: 0 (don't grow)
        Some(0.0), // flex-shrink: 0 (don't shrink)
        None,      // use width
    );
    let item2 = create_flex_item_with_flex(
        &mut engine,
        Some(50.0),
        Some(30.0),
        Some(1.0), // flex-grow: 1 (can grow)
        None,
        None,
    );

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning and sizing
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);

    // Item1 should not grow or shrink, keeping original width
    assert_eq!(w1, 100.0);
    // Item2 should grow to fill remaining space (300 - 150 = 150px available, 50px base + 150px growth)
    assert_eq!(w2, 200.0);

    // Verify positioning
    assert_eq!(x1, 0.0);
    assert_eq!(x2, 100.0);

    // Both items should be on the same line
    assert_eq!(y1, 0.0);
    assert_eq!(y2, 0.0);
}
