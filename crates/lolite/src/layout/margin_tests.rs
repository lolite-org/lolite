use crate::{css_parser::parse_css, layout::asserts::LayoutContextAsserts};

use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

fn div(ctx: &mut LayoutContext, parent: Id, class: &str) -> Id {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let id = Id::from_u64(NEXT.fetch_add(1, Ordering::Relaxed));

    let node = ctx.document.create_node(id, None);
    ctx.document.set_parent(parent, node).unwrap();
    ctx.document
        .set_attribute(node, "class".to_owned(), class.to_owned());
    node
}

#[test]
fn test_margin_override_1() {
    let mut ctx = LayoutContext::new();

    // first margin, then override margin-left
    ctx.style_sheet = parse_css(
        r#"
    .container {
        display: flex;
        width: 400px;
        height: 200px;
        background: gray;
    }

    .child {
        width: 60px;
        height: 40px;
        background: green;
        margin: 20px;
        margin-left: 50px;
    }

    "#,
    )
    .unwrap();

    let root = ctx.document.root_id();
    let container = div(&mut ctx, root, "container");

    let child = div(&mut ctx, container, "child");

    ctx.layout();

    // assert that the bounds are correct
    ctx.assert_node_bounds_eq(container, &Rect::new(0.0, 0.0, 400.0, 200.0));
    ctx.assert_node_bounds_eq(child, &Rect::new(50.0, 20.0, 60.0, 40.0));
}

#[test]
fn test_margin_override_2() {
    let mut ctx = LayoutContext::new();

    // first margin-left, then override margin
    ctx.style_sheet = parse_css(
        r#"
    .container {
        display: flex;
        width: 400px;
        height: 200px;
        background: gray;
    }

    .child {
        width: 60px;
        height: 40px;
        background: green;
        margin-left: 50px;
        margin: 20px;
    }

    "#,
    )
    .unwrap();

    let root = ctx.document.root_id();
    let container = div(&mut ctx, root, "container");

    let child = div(&mut ctx, container, "child");

    ctx.layout();

    // assert that the bounds are correct
    ctx.assert_node_bounds_eq(container, &Rect::new(0.0, 0.0, 400.0, 200.0));
    ctx.assert_node_bounds_eq(child, &Rect::new(20.0, 20.0, 60.0, 40.0));
}
