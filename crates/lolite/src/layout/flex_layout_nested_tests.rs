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
fn test_nested_flex_layout() {
    // expected layout:
    //
    // -------------------------
    // |    flex_container     |
    // ||---------------------||
    // ||| child1 |  child2   ||
    // |||--------|           ||
    // ||| nested |           ||
    // ||| nested |           ||
    // |||        |           ||
    // ||| nested |           ||
    // ||| nested |           ||
    // |||--------|-----------||
    // ||---------------------||
    // -------------------------

    let mut ctx = LayoutContext::new();

    ctx.style_sheet = parse_css(
        r#"
  .flex_container {
    display: flex;
    flex-direction: row;
    width: 400px;
    height: 200px;
  }

  .child1 {
    display: flex;
    flex-direction: column;
    flex-wrap: wrap;
  }

  .grow {
    flex: 1;
  }

  .child2 {
    flex: 1;
    background-color: red;
  }

  .nested_child {
    height: 40px;
    width: 60px;
    background: green;
  }
    "#,
    )
    .unwrap();

    let root = ctx.document.root_id();
    let container = div(&mut ctx, root, "flex_container");

    let child1 = div(&mut ctx, container, "child1");

    let nested1 = div(&mut ctx, child1, "nested_child");
    let nested2 = div(&mut ctx, child1, "nested_child grow");
    let nested3 = div(&mut ctx, child1, "nested_child");
    let nested4 = div(&mut ctx, child1, "nested_child");

    let child2 = div(&mut ctx, container, "child2");

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
