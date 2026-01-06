use crate::layout::{asserts::LayoutContextAsserts, test_html::load_html_test_example};

use super::*;

const HTML: &str = include_str!("./flex_layout_margin_tests.html");

fn setup_margin_demo_ctx(example_id: &str) -> (LayoutContext, Id, Id, Id, Id) {
    let (ctx, nodes_by_id) = load_html_test_example(HTML, example_id);

    let container = nodes_by_id[example_id];

    let child1 = nodes_by_id["child1"];
    let child2 = nodes_by_id["child2"];
    let child3 = nodes_by_id["child3"];

    (ctx, container, child1, child2, child3)
}

#[test]
fn case_1_margin_left_20px() {
    let (ctx, container, child1, child2, child3) = setup_margin_demo_ctx("case1");

    ctx.assert_node_bounds_eq(container, &Rect::new(0.0, 0.0, 400.0, 200.0));
    ctx.assert_node_bounds_eq(child1, &Rect::new(0.0, 0.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child2, &Rect::new(80.0, 0.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child3, &Rect::new(140.0, 0.0, 60.0, 40.0));
}

#[test]
fn case_2_margin_left_auto() {
    let (ctx, container, child1, child2, child3) = setup_margin_demo_ctx("case2");

    ctx.assert_node_bounds_eq(container, &Rect::new(0.0, 0.0, 400.0, 200.0));
    ctx.assert_node_bounds_eq(child1, &Rect::new(0.0, 0.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child2, &Rect::new(280.0, 0.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child3, &Rect::new(340.0, 0.0, 60.0, 40.0));
}

#[test]
fn case_3_margin_20px() {
    let (ctx, container, child1, child2, child3) = setup_margin_demo_ctx("case3");

    ctx.assert_node_bounds_eq(container, &Rect::new(0.0, 0.0, 400.0, 200.0));
    ctx.assert_node_bounds_eq(child1, &Rect::new(0.0, 0.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child2, &Rect::new(80.0, 20.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child3, &Rect::new(160.0, 0.0, 60.0, 40.0));
}

#[test]
fn case_4_margin_auto() {
    let (ctx, container, child1, child2, child3) = setup_margin_demo_ctx("case4");

    ctx.assert_node_bounds_eq(container, &Rect::new(0.0, 0.0, 400.0, 200.0));
    ctx.assert_node_bounds_eq(child1, &Rect::new(0.0, 0.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child2, &Rect::new(170.0, 80.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child3, &Rect::new(340.0, 0.0, 60.0, 40.0));
}

#[test]
fn case_5_margin_0_20px() {
    let (ctx, container, child1, child2, child3) = setup_margin_demo_ctx("case5");

    ctx.assert_node_bounds_eq(container, &Rect::new(0.0, 0.0, 400.0, 200.0));
    ctx.assert_node_bounds_eq(child1, &Rect::new(0.0, 0.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child2, &Rect::new(80.0, 0.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child3, &Rect::new(160.0, 0.0, 60.0, 40.0));
}

#[test]
fn case_6_margin_20px_0() {
    let (ctx, container, child1, child2, child3) = setup_margin_demo_ctx("case6");

    ctx.assert_node_bounds_eq(container, &Rect::new(0.0, 0.0, 400.0, 200.0));
    ctx.assert_node_bounds_eq(child1, &Rect::new(0.0, 0.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child2, &Rect::new(60.0, 20.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child3, &Rect::new(120.0, 0.0, 60.0, 40.0));
}
