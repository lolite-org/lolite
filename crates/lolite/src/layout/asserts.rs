pub trait RectAsserts {
    fn assert_eq(&self, expected: &super::Rect);
}

impl RectAsserts for super::Rect {
    fn assert_eq(&self, expected: &super::Rect) {
        assert!(
            (self.x - expected.x).abs() < 0.001,
            "Expected x: {}, got: {}",
            expected.x,
            self.x
        );
        assert!(
            (self.y - expected.y).abs() < 0.001,
            "Expected y: {}, got: {}",
            expected.y,
            self.y
        );
        assert!(
            (self.width - expected.width).abs() < 0.001,
            "Expected width: {}, got: {}",
            expected.width,
            self.width
        );
        assert!(
            (self.height - expected.height).abs() < 0.001,
            "Expected height: {}, got: {}",
            expected.height,
            self.height
        );
    }
}

pub trait LayoutContextAsserts {
    fn assert_node_bounds_eq(&self, node_id: super::Id, expected: &super::Rect);
}

impl LayoutContextAsserts for super::LayoutContext {
    fn assert_node_bounds_eq(&self, node_id: super::Id, expected: &super::Rect) {
        let node = self
            .document
            .get_node(node_id)
            .unwrap_or_else(|| panic!("Node {:?} not found", node_id));
        let bounds = node.borrow().layout.bounds;
        bounds.assert_eq(expected);
    }
}
