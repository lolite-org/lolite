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

fn setup_margin_demo_ctx(extra_css: &str, child2_class: &str) -> (LayoutContext, Id, Id, Id, Id) {
    let mut ctx = LayoutContext::new();

    ctx.style_sheet = parse_css(&format!(
        r#"
  .flex_container {{
    display: flex;
    flex-direction: row;
    width: 400px;
    height: 200px;
  }}

  .child {{
    display: flex;
    flex-direction: column;
    flex-wrap: wrap;
    width: 60px;
    height: 40px;
  }}

  {extra_css}
"#
    ))
    .unwrap();

    let root = ctx.document.root_id();
    let container = div(&mut ctx, root, "flex_container");

    let child1 = div(&mut ctx, container, "child");
    let child2 = div(&mut ctx, container, &format!("child {child2_class}"));
    let child3 = div(&mut ctx, container, "child");

    (ctx, container, child1, child2, child3)
}

#[test]
#[ignore = "flex layout does not apply margins yet"]
fn case_1_margin_left_20px() {
    // HTML: .margin_left_20px { margin-left: 20px; }
    // Parser note: we only parse `margin`, not `margin-left`.
    // Equivalent: margin: 0 0 0 20px
    let (mut ctx, container, child1, child2, child3) = setup_margin_demo_ctx(
        r#"
  .margin_left_20px {
    margin: 0 0 0 20px;
  }
"#,
        "margin_left_20px",
    );

    ctx.layout();

    ctx.assert_node_bounds_eq(container, &Rect::new(0.0, 0.0, 400.0, 200.0));
    ctx.assert_node_bounds_eq(child1, &Rect::new(0.0, 0.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child2, &Rect::new(80.0, 0.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child3, &Rect::new(140.0, 0.0, 60.0, 40.0));
}

#[test]
#[ignore = "auto margins are not implemented yet (margin auto currently resolves to 0px)"]
fn case_2_margin_left_auto() {
    // HTML: .margin_left_auto { margin-left: auto; }
    // Equivalent shorthand: margin: 0 0 0 auto
    // Flexbox expected behavior: the auto margin absorbs remaining free space.
    let (mut ctx, container, child1, child2, child3) = setup_margin_demo_ctx(
        r#"
  .margin_left_auto {
    margin: 0 0 0 auto;
  }
"#,
        "margin_left_auto",
    );

    ctx.layout();

    ctx.assert_node_bounds_eq(container, &Rect::new(0.0, 0.0, 400.0, 200.0));
    ctx.assert_node_bounds_eq(child1, &Rect::new(0.0, 0.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child2, &Rect::new(280.0, 0.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child3, &Rect::new(340.0, 0.0, 60.0, 40.0));
}

#[test]
#[ignore = "flex layout does not apply margins yet"]
fn case_3_margin_20px() {
    // HTML: .margin_20px { margin: 20px; }
    let (mut ctx, container, child1, child2, child3) = setup_margin_demo_ctx(
        r#"
  .margin_20px {
    margin: 20px;
  }
"#,
        "margin_20px",
    );

    ctx.layout();

    ctx.assert_node_bounds_eq(container, &Rect::new(0.0, 0.0, 400.0, 200.0));
    ctx.assert_node_bounds_eq(child1, &Rect::new(0.0, 0.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child2, &Rect::new(80.0, 20.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child3, &Rect::new(160.0, 0.0, 60.0, 40.0));
}

#[test]
#[ignore = "auto margins are not implemented yet (margin auto currently resolves to 0px)"]
fn case_4_margin_auto() {
    // HTML: .margin_auto { margin: auto; }
    // Flexbox expected behavior: auto margins on the item absorb free space.
    let (mut ctx, container, child1, child2, child3) = setup_margin_demo_ctx(
        r#"
  .margin_auto {
    margin: auto;
  }
"#,
        "margin_auto",
    );

    ctx.layout();

    ctx.assert_node_bounds_eq(container, &Rect::new(0.0, 0.0, 400.0, 200.0));
    ctx.assert_node_bounds_eq(child1, &Rect::new(0.0, 0.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child2, &Rect::new(170.0, 80.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child3, &Rect::new(340.0, 0.0, 60.0, 40.0));
}

#[test]
#[ignore = "flex layout does not apply margins yet"]
fn case_5_margin_0_20px() {
    // HTML: .margin_main_20px { margin: 0 20px; }
    let (mut ctx, container, child1, child2, child3) = setup_margin_demo_ctx(
        r#"
  .margin_main_20px {
    margin: 0 20px;
  }
"#,
        "margin_main_20px",
    );

    ctx.layout();

    ctx.assert_node_bounds_eq(container, &Rect::new(0.0, 0.0, 400.0, 200.0));
    ctx.assert_node_bounds_eq(child1, &Rect::new(0.0, 0.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child2, &Rect::new(80.0, 0.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child3, &Rect::new(160.0, 0.0, 60.0, 40.0));
}

#[test]
#[ignore = "flex layout does not apply margins yet"]
fn case_6_margin_20px_0() {
    // HTML: .margin_cross_20px { margin: 20px 0; }
    let (mut ctx, container, child1, child2, child3) = setup_margin_demo_ctx(
        r#"
  .margin_cross_20px {
    margin: 20px 0;
  }
"#,
        "margin_cross_20px",
    );

    ctx.layout();

    ctx.assert_node_bounds_eq(container, &Rect::new(0.0, 0.0, 400.0, 200.0));
    ctx.assert_node_bounds_eq(child1, &Rect::new(0.0, 0.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child2, &Rect::new(60.0, 20.0, 60.0, 40.0));
    ctx.assert_node_bounds_eq(child3, &Rect::new(120.0, 0.0, 60.0, 40.0));
}
