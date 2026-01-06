use crate::layout::{asserts::LayoutContextAsserts, test_html::load_html_test_example};

use super::*;

const HTML: &str = include_str!("./margin_tests.html");

#[test]
fn test_margin_override_1() {
    let (ctx, nodes_by_id) = load_html_test_example(HTML, "margin-override-1");

    let container = nodes_by_id["margin-override-1"];
    let child = nodes_by_id["child"];

    ctx.assert_node_bounds_eq(container, &Rect::new(0.0, 0.0, 400.0, 200.0));
    ctx.assert_node_bounds_eq(child, &Rect::new(50.0, 20.0, 60.0, 40.0));
}

#[test]
fn test_margin_override_2() {
    let (ctx, nodes_by_id) = load_html_test_example(HTML, "margin-override-2");

    let container = nodes_by_id["margin-override-2"];
    let child = nodes_by_id["child"];

    ctx.assert_node_bounds_eq(container, &Rect::new(0.0, 0.0, 400.0, 200.0));
    ctx.assert_node_bounds_eq(child, &Rect::new(20.0, 20.0, 60.0, 40.0));
}
