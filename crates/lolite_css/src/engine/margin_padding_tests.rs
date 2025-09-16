use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::style::{Display, Extend, FlexDirection, Length, Rule, Selector, Style};

fn next_test_id() -> Id {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    Id::from_u64(NEXT.fetch_add(1, Ordering::Relaxed))
}

// Helper function to create a basic engine setup
fn create_test_engine() -> Engine {
    Engine::new()
}

// Helper function to create a container with specified properties
fn create_container(
    engine: &mut Engine,
    width: Option<f64>,
    height: Option<f64>,
    margin: Option<Extend>,
    padding: Option<Extend>,
) -> Id {
    let container_id = engine.document.create_node(next_test_id(), None);

    // Add a CSS rule for the container
    let class_name = format!("container_{}", container_id.0);
    engine.style_sheet.add_rule(Rule {
        selector: Selector::Class(class_name.clone()),
        declarations: vec![Style {
            display: Display::Flex,
            flex_direction: Some(FlexDirection::Row),
            width: width.map(Length::Px),
            height: height.map(Length::Px),
            margin,
            padding,
            ..Default::default()
        }],
    });

    engine
        .document
        .set_attribute(container_id, "class".to_owned(), class_name);
    container_id
}

// Helper function to create an item with margin and padding
fn create_item_with_spacing(
    engine: &mut Engine,
    width: f64,
    height: f64,
    margin: Option<Extend>,
    padding: Option<Extend>,
) -> Id {
    let item_id = engine
        .document
        .create_node(next_test_id(), Some("item".to_string()));

    // Add a CSS rule for the item
    let class_name = format!("item_{}", item_id.0);
    engine.style_sheet.add_rule(Rule {
        selector: Selector::Class(class_name.clone()),
        declarations: vec![Style {
            width: Some(Length::Px(width)),
            height: Some(Length::Px(height)),
            margin,
            padding,
            ..Default::default()
        }],
    });

    engine
        .document
        .set_attribute(item_id, "class".to_owned(), class_name);
    item_id
}

// Helper function to create a simple item without spacing
fn create_item(engine: &mut Engine, width: f64, height: f64) -> Id {
    create_item_with_spacing(engine, width, height, None, None)
}

// Helper function to get node bounds after layout
fn get_bounds(engine: &Engine, node_id: Id) -> (f64, f64, f64, f64) {
    let node = engine.document.nodes.get(&node_id).unwrap();
    let bounds = &node.borrow().layout.bounds;
    (bounds.x, bounds.y, bounds.width, bounds.height)
}

// Helper function to create uniform spacing
fn uniform_spacing(value: f64) -> Extend {
    Extend {
        top: Length::Px(value),
        right: Length::Px(value),
        bottom: Length::Px(value),
        left: Length::Px(value),
    }
}

// Helper function to create asymmetric spacing
fn asymmetric_spacing(top: f64, right: f64, bottom: f64, left: f64) -> Extend {
    Extend {
        top: Length::Px(top),
        right: Length::Px(right),
        bottom: Length::Px(bottom),
        left: Length::Px(left),
    }
}

// MARGIN TESTS

#[test]
fn test_margin_uniform() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a container
    let container = create_container(&mut engine, Some(300.0), Some(200.0), None, None);
    engine.document.set_parent(root, container).unwrap();

    // Create an item with uniform 20px margin
    let item =
        create_item_with_spacing(&mut engine, 100.0, 50.0, Some(uniform_spacing(20.0)), None);
    engine.document.set_parent(container, item).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning and sizing
    let (x, y, w, h) = get_bounds(&engine, item);

    // Item should be positioned with margin offset
    assert_eq!(x, 20.0); // left margin
    assert_eq!(y, 20.0); // top margin

    // Item content size should remain the same
    assert_eq!(w, 100.0);
    assert_eq!(h, 50.0);
}

#[test]
fn test_margin_asymmetric() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a container
    let container = create_container(&mut engine, Some(300.0), Some(200.0), None, None);
    engine.document.set_parent(root, container).unwrap();

    // Create an item with asymmetric margin: top=10, right=15, bottom=20, left=25
    let item = create_item_with_spacing(
        &mut engine,
        100.0,
        50.0,
        Some(asymmetric_spacing(10.0, 15.0, 20.0, 25.0)),
        None,
    );
    engine.document.set_parent(container, item).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning and sizing
    let (x, y, w, h) = get_bounds(&engine, item);

    // Item should be positioned with margin offset
    assert_eq!(x, 25.0); // left margin
    assert_eq!(y, 10.0); // top margin

    // Item content size should remain the same
    assert_eq!(w, 100.0);
    assert_eq!(h, 50.0);
}

#[test]
fn test_margin_multiple_items_horizontal() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a container
    let container = create_container(&mut engine, Some(400.0), Some(200.0), None, None);
    engine.document.set_parent(root, container).unwrap();

    // Create items with different margins
    let item1 =
        create_item_with_spacing(&mut engine, 80.0, 50.0, Some(uniform_spacing(10.0)), None);
    let item2 = create_item_with_spacing(
        &mut engine,
        60.0,
        50.0,
        Some(asymmetric_spacing(5.0, 20.0, 5.0, 15.0)),
        None,
    );
    let item3 = create_item(&mut engine, 70.0, 50.0); // No margin

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();
    engine.document.set_parent(container, item3).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);
    let (x3, y3, w3, h3) = get_bounds(&engine, item3);

    // Verify sizes remain unchanged
    assert_eq!(w1, 80.0);
    assert_eq!(h1, 50.0);
    assert_eq!(w2, 60.0);
    assert_eq!(h2, 50.0);
    assert_eq!(w3, 70.0);
    assert_eq!(h3, 50.0);

    // Verify horizontal positioning with margins
    assert_eq!(x1, 10.0); // left margin
    assert_eq!(x2, 115.0); // after item1 (10 + 80 + 10) + left margin (15)
    assert_eq!(x3, 195.0); // after item2 (115 + 60 + 20) + no margin

    // Verify vertical positioning with top margins
    assert_eq!(y1, 10.0); // top margin
    assert_eq!(y2, 5.0); // top margin
    assert_eq!(y3, 0.0); // no margin
}

#[test]
fn test_margin_column_direction() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a container with column direction
    let container_id = engine.document.create_node(next_test_id(), None);
    let class_name = format!("container_{}", container_id.0);
    engine.style_sheet.add_rule(Rule {
        selector: Selector::Class(class_name.clone()),
        declarations: vec![Style {
            display: Display::Flex,
            flex_direction: Some(FlexDirection::Column),
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(400.0)),
            ..Default::default()
        }],
    });
    engine
        .document
        .set_attribute(container_id, "class".to_owned(), class_name);
    engine.document.set_parent(root, container_id).unwrap();

    // Create items with margins
    let item1 =
        create_item_with_spacing(&mut engine, 100.0, 60.0, Some(uniform_spacing(15.0)), None);
    let item2 = create_item_with_spacing(
        &mut engine,
        100.0,
        40.0,
        Some(asymmetric_spacing(20.0, 10.0, 25.0, 5.0)),
        None,
    );

    engine.document.set_parent(container_id, item1).unwrap();
    engine.document.set_parent(container_id, item2).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);

    // Verify sizes
    assert_eq!(w1, 100.0);
    assert_eq!(h1, 60.0);
    assert_eq!(w2, 100.0);
    assert_eq!(h2, 40.0);

    // Verify positioning
    assert_eq!(x1, 15.0); // left margin
    assert_eq!(y1, 15.0); // top margin
    assert_eq!(x2, 5.0); // left margin
    assert_eq!(y2, 110.0); // after item1 (15 + 60 + 15) + top margin (20)
}

// PADDING TESTS

#[test]
fn test_padding_uniform() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a container with padding
    let container = create_container(
        &mut engine,
        Some(300.0),
        Some(200.0),
        None,
        Some(uniform_spacing(20.0)),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create an item
    let item = create_item(&mut engine, 100.0, 50.0);
    engine.document.set_parent(container, item).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning
    let (x, y, w, h) = get_bounds(&engine, item);

    // Item should be positioned inside the padding
    assert_eq!(x, 20.0); // padding-left
    assert_eq!(y, 20.0); // padding-top

    // Item size should remain the same
    assert_eq!(w, 100.0);
    assert_eq!(h, 50.0);
}

#[test]
fn test_padding_asymmetric() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a container with asymmetric padding: top=15, right=25, bottom=35, left=45
    let container = create_container(
        &mut engine,
        Some(300.0),
        Some(200.0),
        None,
        Some(asymmetric_spacing(15.0, 25.0, 35.0, 45.0)),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create an item
    let item = create_item(&mut engine, 100.0, 50.0);
    engine.document.set_parent(container, item).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning
    let (x, y, w, h) = get_bounds(&engine, item);

    // Item should be positioned inside the padding
    assert_eq!(x, 45.0); // padding-left
    assert_eq!(y, 15.0); // padding-top

    // Item size should remain the same
    assert_eq!(w, 100.0);
    assert_eq!(h, 50.0);
}

#[test]
fn test_padding_multiple_items() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a container with padding
    let container = create_container(
        &mut engine,
        Some(400.0),
        Some(200.0),
        None,
        Some(uniform_spacing(20.0)),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create multiple items
    let item1 = create_item(&mut engine, 80.0, 50.0);
    let item2 = create_item(&mut engine, 60.0, 50.0);
    let item3 = create_item(&mut engine, 70.0, 50.0);

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();
    engine.document.set_parent(container, item3).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);
    let (x3, y3, w3, h3) = get_bounds(&engine, item3);

    // Verify sizes
    assert_eq!(w1, 80.0);
    assert_eq!(h1, 50.0);
    assert_eq!(w2, 60.0);
    assert_eq!(h2, 50.0);
    assert_eq!(w3, 70.0);
    assert_eq!(h3, 50.0);

    // All items should be positioned inside the padding area
    assert_eq!(y1, 20.0); // padding-top
    assert_eq!(y2, 20.0); // padding-top
    assert_eq!(y3, 20.0); // padding-top

    // Horizontal positioning should account for padding
    assert_eq!(x1, 20.0); // padding-left
    assert_eq!(x2, 100.0); // after item1 (20 + 80)
    assert_eq!(x3, 160.0); // after item2 (100 + 60)
}

#[test]
fn test_padding_column_direction() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a container with column direction and padding
    let container_id = engine.document.create_node(next_test_id(), None);
    let class_name = format!("container_{}", container_id.0);
    engine.style_sheet.add_rule(Rule {
        selector: Selector::Class(class_name.clone()),
        declarations: vec![Style {
            display: Display::Flex,
            flex_direction: Some(FlexDirection::Column),
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(400.0)),
            padding: Some(uniform_spacing(30.0)),
            ..Default::default()
        }],
    });
    engine
        .document
        .set_attribute(container_id, "class".to_owned(), class_name);
    engine.document.set_parent(root, container_id).unwrap();

    // Create items
    let item1 = create_item(&mut engine, 100.0, 60.0);
    let item2 = create_item(&mut engine, 100.0, 40.0);

    engine.document.set_parent(container_id, item1).unwrap();
    engine.document.set_parent(container_id, item2).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);

    // Verify sizes
    assert_eq!(w1, 100.0);
    assert_eq!(h1, 60.0);
    assert_eq!(w2, 100.0);
    assert_eq!(h2, 40.0);

    // Verify positioning with padding
    assert_eq!(x1, 30.0); // padding-left
    assert_eq!(y1, 30.0); // padding-top
    assert_eq!(x2, 30.0); // padding-left
    assert_eq!(y2, 90.0); // after item1 (30 + 60)
}

// COMBINED MARGIN AND PADDING TESTS

#[test]
fn test_margin_and_padding_combined() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a container with padding
    let container = create_container(
        &mut engine,
        Some(400.0),
        Some(300.0),
        None,
        Some(uniform_spacing(20.0)),
    );
    engine.document.set_parent(root, container).unwrap();

    // Create an item with margin
    let item =
        create_item_with_spacing(&mut engine, 100.0, 50.0, Some(uniform_spacing(15.0)), None);
    engine.document.set_parent(container, item).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning
    let (x, y, w, h) = get_bounds(&engine, item);

    // Item should be positioned with both container padding and item margin
    assert_eq!(x, 35.0); // container padding-left (20) + item margin-left (15)
    assert_eq!(y, 35.0); // container padding-top (20) + item margin-top (15)

    // Item size should remain the same
    assert_eq!(w, 100.0);
    assert_eq!(h, 50.0);
}

#[ignore]
#[test]
fn test_nested_containers_with_spacing() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create outer container with margin and padding
    let outer_container = create_container(
        &mut engine,
        Some(400.0),
        Some(300.0),
        Some(uniform_spacing(10.0)), // margin
        Some(uniform_spacing(15.0)), // padding
    );
    engine.document.set_parent(root, outer_container).unwrap();

    // Create inner container with margin and padding
    let inner_container = create_container(
        &mut engine,
        Some(200.0),
        Some(150.0),
        Some(uniform_spacing(5.0)),  // margin
        Some(uniform_spacing(20.0)), // padding
    );
    engine
        .document
        .set_parent(outer_container, inner_container)
        .unwrap();

    // Create an item
    let item = create_item(&mut engine, 80.0, 40.0);
    engine.document.set_parent(inner_container, item).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning
    let (outer_x, outer_y, outer_w, outer_h) = get_bounds(&engine, outer_container);
    let (inner_x, inner_y, inner_w, inner_h) = get_bounds(&engine, inner_container);
    let (item_x, item_y, item_w, item_h) = get_bounds(&engine, item);

    // Outer container positioning
    assert_eq!(outer_x, 10.0); // margin-left
    assert_eq!(outer_y, 10.0); // margin-top
    assert_eq!(outer_w, 400.0);
    assert_eq!(outer_h, 300.0);

    // Inner container positioning (inside outer container's padding + its own margin)
    assert_eq!(inner_x, 30.0); // outer padding-left (15) + inner margin-left (5) + outer margin-left (10)
    assert_eq!(inner_y, 30.0); // outer padding-top (15) + inner margin-top (5) + outer margin-top (10)
    assert_eq!(inner_w, 200.0);
    assert_eq!(inner_h, 150.0);

    // Item positioning (inside inner container's padding)
    assert_eq!(item_x, 50.0); // inner container x (30) + inner padding-left (20)
    assert_eq!(item_y, 50.0); // inner container y (30) + inner padding-top (20)
    assert_eq!(item_w, 80.0);
    assert_eq!(item_h, 40.0);
}

#[test]
fn test_item_with_padding() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a container
    let container = create_container(&mut engine, Some(300.0), Some(200.0), None, None);
    engine.document.set_parent(root, container).unwrap();

    // Create an item with padding (this should affect the item's content area)
    let item =
        create_item_with_spacing(&mut engine, 100.0, 50.0, None, Some(uniform_spacing(10.0)));
    engine.document.set_parent(container, item).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning and sizing
    let (x, y, w, h) = get_bounds(&engine, item);

    // Item should be positioned normally
    assert_eq!(x, 0.0);
    assert_eq!(y, 0.0);

    // Item size should include padding in the total bounds
    // The content area is 100x50, but the total bounds should include padding
    // This depends on how we implement padding - whether it's internal or adds to size
    // For now, let's assume padding is internal (reduces content area)
    assert_eq!(w, 100.0);
    assert_eq!(h, 50.0);
}

#[test]
fn test_margin_collapse_prevention() {
    let mut engine = create_test_engine();
    let root = engine.document.root_id();

    // Create a container
    let container = create_container(&mut engine, Some(400.0), Some(200.0), None, None);
    engine.document.set_parent(root, container).unwrap();

    // Create two adjacent items with margins
    let item1 = create_item_with_spacing(
        &mut engine,
        100.0,
        50.0,
        Some(asymmetric_spacing(10.0, 15.0, 20.0, 5.0)), // bottom margin: 20px
        None,
    );
    let item2 = create_item_with_spacing(
        &mut engine,
        100.0,
        50.0,
        Some(asymmetric_spacing(25.0, 10.0, 15.0, 8.0)), // top margin: 25px
        None,
    );

    engine.document.set_parent(container, item1).unwrap();
    engine.document.set_parent(container, item2).unwrap();

    // Run layout
    engine.layout();

    // Verify positioning
    let (x1, y1, w1, h1) = get_bounds(&engine, item1);
    let (x2, y2, w2, h2) = get_bounds(&engine, item2);

    // Verify sizes
    assert_eq!(w1, 100.0);
    assert_eq!(h1, 50.0);
    assert_eq!(w2, 100.0);
    assert_eq!(h2, 50.0);

    // In flexbox, margins don't collapse, so both margins should be applied
    assert_eq!(x1, 5.0); // left margin
    assert_eq!(y1, 10.0); // top margin
    assert_eq!(x2, 128.0); // after item1 (5 + 100 + 15) + left margin (8)
    assert_eq!(y2, 25.0); // top margin (margins don't collapse in flexbox)
}
