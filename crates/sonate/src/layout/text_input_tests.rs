use super::*;
use crate::css_parser::parse_css;
use crate::style::Widget;
use std::sync::atomic::{AtomicU64, Ordering};

fn next_test_id() -> Id {
    static NEXT: AtomicU64 = AtomicU64::new(10_000);
    Id::from_u64(NEXT.fetch_add(1, Ordering::Relaxed))
}

#[test]
fn text_input_layout_initializes_value_and_focus_state() {
    let mut ctx = LayoutContext::new();
    ctx.style_sheet = parse_css(
        r#"
        input {
            -sonate-widget: text-input;
            width: 120px;
        }
    "#,
    )
    .expect("stylesheet should parse");

    let id = ctx.document.create_node(next_test_id(), None);
    ctx.document
        .set_parent(ctx.document.root_id(), id)
        .expect("parent should be set");
    ctx.document
        .set_attribute(id, "tag".to_string(), "input".to_string());
    ctx.document
        .set_attribute(id, "value".to_string(), "hello".to_string());

    ctx.layout();

    let node = ctx.document.get_node(id).expect("node should exist");
    let node = node.borrow();
    assert_eq!(node.layout.style.widget, Some(Widget::TextInput));
    assert_eq!(node.text.as_deref(), Some("hello"));
    assert_eq!(
        node.text_input_state.as_ref().map(|state| state.caret),
        Some(5)
    );
    assert_eq!(node.layout.bounds.width, 120.0);
}

#[test]
fn text_input_edit_commands_update_text_and_caret() {
    let mut ctx = LayoutContext::new();
    let id = ctx.document.create_node(next_test_id(), None);
    let node = ctx.document.get_node(id).expect("node should exist");

    {
        let mut node = node.borrow_mut();
        node.text_input_state = Some(TextInputState::default());
        node.text = Some("ab".to_string());
    }

    assert!(ctx.set_focused_text_input(Some(id)));
    assert!(ctx.insert_text_at_focus("c"));
    assert!(ctx.move_caret_left());
    assert!(ctx.delete_backward_at_focus());

    let node = ctx.document.get_node(id).expect("node should exist");
    let node = node.borrow();
    assert_eq!(node.text.as_deref(), Some("ac"));
    assert_eq!(node.attributes.get("value").map(String::as_str), Some("ac"));
    assert_eq!(
        node.text_input_state.as_ref().map(|state| state.caret),
        Some(1)
    );
    assert!(node
        .text_input_state
        .as_ref()
        .is_some_and(|state| state.focused));
}
