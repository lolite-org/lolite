use crate::layout::{asserts::LayoutContextAsserts, test_html::load_html_test_example};

use super::*;

const HTML: &str = include_str!("./flex_layout_nested_tests.html");

#[test]
fn test_nested_flex_layout() {
    let (mut ctx, nodes_by_id) = load_html_test_example(HTML, "example");

    let container = nodes_by_id["example"];
    let child1 = nodes_by_id["child1"];
    let nested1 = nodes_by_id["nested_child_1"];
    let nested2 = nodes_by_id["nested_child_2"];
    let nested3 = nodes_by_id["nested_child_3"];
    let nested4 = nodes_by_id["nested_child_4"];
    let child2 = nodes_by_id["child2"];

    ctx.layout();

    // assert that the bounds are correct
    ctx.assert_node_bounds_eq(container, &Rect::new(0.0, 0.0, 400.0, 200.0));
    ctx.assert_node_bounds_eq(child1, &Rect::new(0.0, 0.0, 60.0, 200.0));
    ctx.assert_node_bounds_eq(nested1, &Rect::new(0.0, 0.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(nested2, &Rect::new(0.0, 40.0, 60.0, 80.0));
    ctx.assert_node_bounds_eq(nested3, &Rect::new(0.0, 120.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(nested4, &Rect::new(0.0, 160.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child2, &Rect::new(60.0, 0.0, 340.0, 200.0));
}
