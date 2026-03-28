use super::*;
use crate::style::{Length, Overflow, Style};
use std::sync::Arc;

fn make_id(n: u64) -> Id {
    Id::from_u64(10_000 + n)
}

/// Helper: build a simple tree with a scrollable container and children.
fn setup_scroll_ctx() -> (LayoutContext, Id, Id, Id) {
    let mut ctx = LayoutContext::new();
    let root_id = ctx.document.root_id();

    let container_id = ctx.document.create_node(make_id(1), None);
    let child_a = ctx.document.create_node(make_id(2), None);
    let child_b = ctx.document.create_node(make_id(3), None);

    ctx.document.set_parent(root_id, container_id).unwrap();
    ctx.document.set_parent(container_id, child_a).unwrap();
    ctx.document.set_parent(container_id, child_b).unwrap();

    // Give the container an overflow: scroll style
    {
        let node = ctx.document.get_node(container_id).unwrap();
        let mut nb = node.borrow_mut();
        let mut style = Style::default();
        style.overflow = Some(Overflow::Scroll);
        style.width = Some(Length::Px(200.0));
        style.height = Some(Length::Px(100.0));
        nb.layout.style = Arc::new(style);
    }

    (ctx, container_id, child_a, child_b)
}

// ─── Cleanup on Node Removal ───────────────────────────────────────────────

#[test]
fn scroll_state_cleaned_up_when_node_removed() {
    let (mut ctx, container_id, _child_a, _child_b) = setup_scroll_ctx();

    // Simulate scroll on the container
    ctx.scroll_state.insert(
        container_id,
        ScrollState {
            scroll_x: 0.0,
            scroll_y: 50.0,
        },
    );

    assert!(ctx.scroll_state.contains_key(&container_id));

    // Remove the container (should remove its scroll state)
    ctx.remove_node(container_id);

    assert!(
        !ctx.scroll_state.contains_key(&container_id),
        "scroll state should be removed when the node is removed"
    );
}

#[test]
fn scroll_state_cleaned_up_for_descendants_when_parent_removed() {
    let (mut ctx, container_id, child_a, child_b) = setup_scroll_ctx();

    // Add scroll state to the container and both children
    ctx.scroll_state.insert(
        container_id,
        ScrollState {
            scroll_x: 10.0,
            scroll_y: 20.0,
        },
    );
    ctx.scroll_state.insert(
        child_a,
        ScrollState {
            scroll_x: 0.0,
            scroll_y: 5.0,
        },
    );
    ctx.scroll_state.insert(
        child_b,
        ScrollState {
            scroll_x: 0.0,
            scroll_y: 3.0,
        },
    );

    assert_eq!(ctx.scroll_state.len(), 3);

    // Removing the container should also clean up children's scroll state
    ctx.remove_node(container_id);

    assert!(
        ctx.scroll_state.is_empty(),
        "all descendant scroll states should be removed"
    );
}

#[test]
fn removing_leaf_leaves_other_scroll_state_intact() {
    let (mut ctx, container_id, child_a, _child_b) = setup_scroll_ctx();

    ctx.scroll_state.insert(
        container_id,
        ScrollState {
            scroll_x: 0.0,
            scroll_y: 50.0,
        },
    );
    ctx.scroll_state.insert(
        child_a,
        ScrollState {
            scroll_x: 0.0,
            scroll_y: 10.0,
        },
    );

    // Remove only child_a
    ctx.remove_node(child_a);

    assert!(
        !ctx.scroll_state.contains_key(&child_a),
        "removed node's scroll state should be gone"
    );
    assert!(
        ctx.scroll_state.contains_key(&container_id),
        "unrelated scroll state should remain"
    );
}

// ─── Add and Remove Nodes ──────────────────────────────────────────────────

#[test]
fn add_nodes_scroll_then_remove_one_by_one() {
    let mut ctx = LayoutContext::new();
    let root_id = ctx.document.root_id();

    let a = ctx.document.create_node(make_id(200), None);
    let b = ctx.document.create_node(make_id(201), None);
    ctx.document.set_parent(root_id, a).unwrap();
    ctx.document.set_parent(root_id, b).unwrap();

    ctx.scroll_state.insert(
        a,
        ScrollState {
            scroll_x: 0.0,
            scroll_y: 1.0,
        },
    );
    ctx.scroll_state.insert(
        b,
        ScrollState {
            scroll_x: 2.0,
            scroll_y: 0.0,
        },
    );

    assert_eq!(ctx.scroll_state.len(), 2);

    ctx.remove_node(a);
    assert_eq!(ctx.scroll_state.len(), 1);
    assert!(!ctx.scroll_state.contains_key(&a));
    assert!(ctx.scroll_state.contains_key(&b));

    ctx.remove_node(b);
    assert!(ctx.scroll_state.is_empty());
}

#[test]
fn add_deep_tree_then_remove_subtree() {
    let mut ctx = LayoutContext::new();
    let root_id = ctx.document.root_id();

    let parent = ctx.document.create_node(make_id(300), None);
    let child = ctx.document.create_node(make_id(301), None);
    let grandchild = ctx.document.create_node(make_id(302), None);
    let sibling = ctx.document.create_node(make_id(303), None);

    ctx.document.set_parent(root_id, parent).unwrap();
    ctx.document.set_parent(parent, child).unwrap();
    ctx.document.set_parent(child, grandchild).unwrap();
    ctx.document.set_parent(root_id, sibling).unwrap();

    // Give scroll state to several nodes
    ctx.scroll_state.insert(
        parent,
        ScrollState {
            scroll_x: 0.0,
            scroll_y: 10.0,
        },
    );
    ctx.scroll_state.insert(
        grandchild,
        ScrollState {
            scroll_x: 5.0,
            scroll_y: 0.0,
        },
    );
    ctx.scroll_state.insert(
        sibling,
        ScrollState {
            scroll_x: 1.0,
            scroll_y: 1.0,
        },
    );

    assert_eq!(ctx.scroll_state.len(), 3);

    // Remove the parent subtree — should take parent and grandchild scroll state
    ctx.remove_node(parent);

    assert_eq!(ctx.scroll_state.len(), 1);
    assert!(!ctx.scroll_state.contains_key(&parent));
    assert!(!ctx.scroll_state.contains_key(&grandchild));
    assert!(
        ctx.scroll_state.contains_key(&sibling),
        "sibling scroll state should survive"
    );
}

// ─── Drop / Destroy ────────────────────────────────────────────────────────

#[test]
fn scroll_state_dropped_with_layout_context() {
    // LayoutContext owns the HashMap<Id, ScrollState> directly.
    // When it is dropped, all scroll state is freed. This test
    // verifies the field exists and is populated, then drops the context.
    let mut ctx = LayoutContext::new();
    let root_id = ctx.document.root_id();
    let node_id = ctx.document.create_node(make_id(300), None);
    ctx.document.set_parent(root_id, node_id).unwrap();

    ctx.scroll_state.insert(
        node_id,
        ScrollState {
            scroll_x: 0.0,
            scroll_y: 42.0,
        },
    );
    assert_eq!(ctx.scroll_state.len(), 1);

    drop(ctx);
    // Nothing to assert — the point is that drop completes without issues
    // and that the HashMap is owned (not leaked behind an Rc or global).
}

// ─── Scroll offset is applied during layout ────────────────────────────────

#[test]
fn scroll_offset_shifts_children() {
    let (mut ctx, container_id, child_a, child_b) = setup_scroll_ctx();

    // Set a scroll offset
    ctx.scroll_state.insert(
        container_id,
        ScrollState {
            scroll_x: 0.0,
            scroll_y: 30.0,
        },
    );

    ctx.layout();

    let child_a_node = ctx.document.get_node(child_a).unwrap();
    let child_b_node = ctx.document.get_node(child_b).unwrap();

    let a_y = child_a_node.borrow().layout.bounds.y;
    let b_y = child_b_node.borrow().layout.bounds.y;

    // Now layout without scroll offset for comparison
    ctx.scroll_state.remove(&container_id);
    ctx.layout();

    let a_y_no_scroll = child_a_node.borrow().layout.bounds.y;
    let b_y_no_scroll = child_b_node.borrow().layout.bounds.y;

    assert!(
        (a_y - (a_y_no_scroll - 30.0)).abs() < 0.001,
        "child A should be shifted up by scroll offset: got {a_y}, expected {}",
        a_y_no_scroll - 30.0
    );
    assert!(
        (b_y - (b_y_no_scroll - 30.0)).abs() < 0.001,
        "child B should be shifted up by scroll offset: got {b_y}, expected {}",
        b_y_no_scroll - 30.0
    );
}
