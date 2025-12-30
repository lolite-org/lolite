use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

fn next_test_id() -> Id {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    Id::from_u64(NEXT.fetch_add(1, Ordering::Relaxed))
}

#[test]
fn test_point_in_bounds() {
    let engine = Engine::new();
    let bounds = Rect {
        x: 10.0,
        y: 20.0,
        width: 100.0,
        height: 50.0,
    };

    // Point inside bounds
    assert!(engine.point_in_bounds(&bounds, 50.0, 30.0));
    assert!(engine.point_in_bounds(&bounds, 10.0, 20.0)); // Top-left corner
    assert!(engine.point_in_bounds(&bounds, 110.0, 70.0)); // Bottom-right corner

    // Point outside bounds
    assert!(!engine.point_in_bounds(&bounds, 5.0, 30.0)); // Left of bounds
    assert!(!engine.point_in_bounds(&bounds, 150.0, 30.0)); // Right of bounds
    assert!(!engine.point_in_bounds(&bounds, 50.0, 10.0)); // Above bounds
    assert!(!engine.point_in_bounds(&bounds, 50.0, 80.0)); // Below bounds
}

#[test]
fn test_find_element_at_position_single_element() {
    let engine = Engine::new();
    let root_id = engine.document.root_id();

    // Set bounds for root element
    {
        let root = engine.document.root_node();
        let mut root_borrow = root.borrow_mut();
        root_borrow.layout.bounds = Rect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 100.0,
        };
    }

    // Test point inside root
    let result = engine.find_element_at_position(50.0, 50.0);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], root_id);

    // Test point outside root
    let result = engine.find_element_at_position(250.0, 50.0);
    assert_eq!(result.len(), 0);
}

#[test]
fn test_find_element_at_position_nested_elements() {
    let mut engine = Engine::new();
    let root_id = engine.document.root_id();

    // Create child elements
    let child1_id = engine
        .document
        .create_node(next_test_id(), Some("child1".to_string()));
    let child2_id = engine
        .document
        .create_node(next_test_id(), Some("child2".to_string()));
    let grandchild_id = engine
        .document
        .create_node(next_test_id(), Some("grandchild".to_string()));

    // Set up parent-child relationships
    engine.document.set_parent(root_id, child1_id).unwrap();
    engine.document.set_parent(root_id, child2_id).unwrap();
    engine
        .document
        .set_parent(child1_id, grandchild_id)
        .unwrap();

    // Set bounds for all elements
    {
        let root = engine.document.root_node();
        let mut root_borrow = root.borrow_mut();
        root_borrow.layout.bounds = Rect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 200.0,
        };
    }

    {
        let child1 = engine.document.nodes.get(&child1_id).unwrap();
        let mut child1_borrow = child1.borrow_mut();
        child1_borrow.layout.bounds = Rect {
            x: 10.0,
            y: 10.0,
            width: 100.0,
            height: 100.0,
        };
    }

    {
        let child2 = engine.document.nodes.get(&child2_id).unwrap();
        let mut child2_borrow = child2.borrow_mut();
        child2_borrow.layout.bounds = Rect {
            x: 120.0,
            y: 10.0,
            width: 70.0,
            height: 70.0,
        };
    }

    {
        let grandchild = engine.document.nodes.get(&grandchild_id).unwrap();
        let mut grandchild_borrow = grandchild.borrow_mut();
        grandchild_borrow.layout.bounds = Rect {
            x: 20.0,
            y: 20.0,
            width: 50.0,
            height: 50.0,
        };
    }

    // Test clicking on grandchild - should return [grandchild, child1, root]
    let result = engine.find_element_at_position(40.0, 40.0);
    assert_eq!(result.len(), 3);
    assert_eq!(result[0], grandchild_id);
    assert_eq!(result[1], child1_id);
    assert_eq!(result[2], root_id);

    // Test clicking on child1 but outside grandchild - should return [child1, root]
    let result = engine.find_element_at_position(80.0, 80.0);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], child1_id);
    assert_eq!(result[1], root_id);

    // Test clicking on child2 - should return [child2, root]
    let result = engine.find_element_at_position(150.0, 40.0);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], child2_id);
    assert_eq!(result[1], root_id);

    // Test clicking on root but outside children - should return [root]
    let result = engine.find_element_at_position(5.0, 5.0);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], root_id);

    // Test clicking outside all elements
    let result = engine.find_element_at_position(250.0, 250.0);
    assert_eq!(result.len(), 0);
}

#[test]
fn test_find_element_at_position_overlapping_siblings() {
    let mut engine = Engine::new();
    let root_id = engine.document.root_id();

    // Create two overlapping child elements
    let child1_id = engine
        .document
        .create_node(next_test_id(), Some("child1".to_string()));
    let child2_id = engine
        .document
        .create_node(next_test_id(), Some("child2".to_string()));

    // Set up parent-child relationships
    engine.document.set_parent(root_id, child1_id).unwrap();
    engine.document.set_parent(root_id, child2_id).unwrap();

    // Set bounds for all elements (child2 overlaps child1)
    {
        let root = engine.document.root_node();
        let mut root_borrow = root.borrow_mut();
        root_borrow.layout.bounds = Rect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 200.0,
        };
    }

    {
        let child1 = engine.document.nodes.get(&child1_id).unwrap();
        let mut child1_borrow = child1.borrow_mut();
        child1_borrow.layout.bounds = Rect {
            x: 10.0,
            y: 10.0,
            width: 100.0,
            height: 100.0,
        };
    }

    {
        let child2 = engine.document.nodes.get(&child2_id).unwrap();
        let mut child2_borrow = child2.borrow_mut();
        child2_borrow.layout.bounds = Rect {
            x: 50.0,
            y: 50.0,
            width: 100.0,
            height: 100.0,
        };
    }

    // Test clicking in overlapping area - should hit child2 (last child, rendered on top)
    let result = engine.find_element_at_position(80.0, 80.0);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], child2_id);
    assert_eq!(result[1], root_id);

    // Test clicking in child1 only area
    let result = engine.find_element_at_position(30.0, 30.0);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], child1_id);
    assert_eq!(result[1], root_id);

    // Test clicking in child2 only area
    let result = engine.find_element_at_position(140.0, 140.0);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], child2_id);
    assert_eq!(result[1], root_id);
}
