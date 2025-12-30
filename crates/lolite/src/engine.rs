use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc};

use crate::{
    flex_layout::FlexLayoutEngine,
    style::{Selector, Style, StyleSheet},
};

#[derive(Default)]
pub struct Layout {
    pub bounds: Rect,
    pub style: Arc<Style>,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Default)]
#[allow(unused)]
pub struct Node {
    pub id: Id,
    pub text: Option<String>,
    pub attributes: HashMap<String, String>,
    pub children: Vec<Rc<RefCell<Node>>>,
    pub parent: Option<Id>, // Add parent member
    // modified when layouting
    pub layout: Layout,
}

impl Node {
    pub fn new(id: Id, text: Option<String>) -> Self {
        Self {
            id,
            text,
            ..Default::default()
        }
    }

    pub fn add_child(&mut self, child: Rc<RefCell<Node>>) {
        self.children.push(child);
    }
}

#[derive(Clone, Copy, Default, Debug, Eq, Hash, PartialEq)]
pub struct Id(u64);

impl Id {
    pub fn value(&self) -> u64 {
        self.0
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }

    pub fn from_u64(value: u64) -> Self {
        Id(value)
    }
}

pub struct Document {
    #[allow(unused)]
    root: Rc<RefCell<Node>>,
    nodes: HashMap<Id, Rc<RefCell<Node>>>,
}

impl Document {
    pub fn new() -> Self {
        let root = Rc::new(RefCell::new(Node::new(Id(0), None)));
        let mut nodes = HashMap::new();
        nodes.insert(Id(0), root.clone());
        Self { root, nodes }
    }

    pub fn create_node(&mut self, id: Id, text: Option<String>) -> Id {
        let node = Rc::new(RefCell::new(Node::new(id, text)));
        self.nodes.insert(id, node);
        id
    }

    pub fn set_parent(&mut self, parent_id: Id, child_id: Id) -> Result<(), &str> {
        // Check if the parent and child are the same
        if parent_id == child_id {
            return Err("Parent and child cannot be the same");
        }

        let child = self
            .nodes
            .get(&child_id)
            .ok_or("Child node not found")?
            .clone();

        // Check if the child is already a child of the parent
        if child.borrow().parent == Some(parent_id) {
            return Ok(());
        }

        let parent = self.nodes.get(&parent_id).ok_or("Parent node not found")?;

        // Remove the child from its previous parent
        if let Some(old_parent_id) = child.borrow().parent {
            if let Some(old_parent) = self.nodes.get(&old_parent_id) {
                old_parent
                    .borrow_mut()
                    .children
                    .retain(|c| c.borrow().id != child_id);
            }
        }

        // Set the new parent
        child.borrow_mut().parent = Some(parent_id);
        parent.borrow_mut().add_child(child);
        Ok(())
    }

    pub fn set_attribute(&mut self, node_id: Id, key: String, value: String) {
        if let Some(node) = self.nodes.get(&node_id) {
            node.borrow_mut().attributes.insert(key, value);
        }
    }

    #[allow(unused)]
    pub fn get_attribute(&self, node_id: Id, key: String) -> Option<String> {
        self.nodes
            .get(&node_id)
            .map(|node| node.borrow().attributes.get(&key).cloned())
            .flatten()
    }

    #[allow(unused)]
    pub fn root_id(&self) -> Id {
        Id(0)
    }

    pub fn root_node(&self) -> Rc<RefCell<Node>> {
        self.root.clone()
    }

    #[allow(unused)]
    pub fn get_node(&self, id: Id) -> Option<Rc<RefCell<Node>>> {
        self.nodes.get(&id).cloned()
    }
}

pub struct Engine {
    pub document: Document,
    pub style_sheet: StyleSheet,
    flex_layout_engine: FlexLayoutEngine,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            document: Document::new(),
            style_sheet: StyleSheet::new(),
            flex_layout_engine: FlexLayoutEngine::new(),
        }
    }

    /// Find the element at the given position (x, y).
    /// Returns a Vec<Id> where the first element is the topmost element at the position,
    /// and subsequent elements are its parents up to the root.
    /// This enables event bubbling by providing the full parent chain.
    pub fn find_element_at_position(&self, x: f64, y: f64) -> Vec<Id> {
        let mut result = Vec::new();

        // Start from the root and find the deepest element containing the point
        if let Some(topmost_id) =
            self.find_topmost_element_at_position(self.document.root.clone(), x, y)
        {
            // Build the parent chain starting from the topmost element
            let mut current_id = topmost_id;
            result.push(current_id);

            // Walk up the parent chain
            while let Some(node) = self.document.nodes.get(&current_id) {
                if let Some(parent_id) = node.borrow().parent {
                    result.push(parent_id);
                    current_id = parent_id;
                } else {
                    break;
                }
            }
        }

        result
    }

    /// Recursively find the topmost (deepest) element that contains the given position
    fn find_topmost_element_at_position(
        &self,
        node: Rc<RefCell<Node>>,
        x: f64,
        y: f64,
    ) -> Option<Id> {
        let node_borrow = node.borrow();

        // Check if the point is within this node's bounds
        if !self.point_in_bounds(&node_borrow.layout.bounds, x, y) {
            return None;
        }

        let node_id = node_borrow.id;

        // Check children in reverse order (later children are rendered on top)
        for child in node_borrow.children.iter().rev() {
            if let Some(child_id) = self.find_topmost_element_at_position(child.clone(), x, y) {
                return Some(child_id);
            }
        }

        // No child contains the point, so this node is the topmost
        Some(node_id)
    }

    /// Check if a point (x, y) is within the given bounds
    fn point_in_bounds(&self, bounds: &Rect, x: f64, y: f64) -> bool {
        x >= bounds.x
            && x <= bounds.x + bounds.width
            && y >= bounds.y
            && y <= bounds.y + bounds.height
    }

    pub fn layout(&mut self) {
        self.layout_node(self.document.root.clone(), 0.0, 0.0);
    }

    pub fn layout_node(&self, node: Rc<RefCell<Node>>, x: f64, y: f64) {
        // Get style for this node - merge existing style with CSS rules
        let style = {
            let node_borrow = node.borrow();
            // Start with existing style as base (this preserves manually set properties like flex_wrap)
            let mut style = node_borrow.layout.style.as_ref().clone();

            // Apply CSS rules on top of existing style
            if let Some(class) = node_borrow.attributes.get("class") {
                let selector = Selector::Class(class.clone());
                let rule = self
                    .style_sheet
                    .rules
                    .iter()
                    .find(|rule| rule.selector == selector);

                if let Some(rule) = rule {
                    for declaration in &rule.declarations {
                        style.merge(declaration);
                    }
                }
            }
            style
        };

        // Set position (margins will be applied by flex layout engine for flex items)
        {
            let mut node_borrow = node.borrow_mut();
            node_borrow.layout.bounds.x = x;
            node_borrow.layout.bounds.y = y;
        }

        let is_leaf = node.borrow().children.is_empty();

        if is_leaf {
            // Leaf node - use specified dimensions or defaults
            let mut node_borrow = node.borrow_mut();
            node_borrow.layout.bounds.width =
                style.width.as_ref().map(|w| w.to_px()).unwrap_or(100.0);
            node_borrow.layout.bounds.height =
                style.height.as_ref().map(|h| h.to_px()).unwrap_or(30.0);
            node_borrow.layout.style = Arc::new(style);
        } else {
            // Container node - handle flexbox layout
            let container_width = style.width.as_ref().map(|w| w.to_px()).unwrap_or(300.0);
            let container_height = style.height.as_ref().map(|h| h.to_px()).unwrap_or(200.0);

            // Set container dimensions
            {
                let mut node_borrow = node.borrow_mut();
                node_borrow.layout.bounds.width = container_width;
                node_borrow.layout.bounds.height = container_height;
                node_borrow.layout.style = Arc::new(style.clone());
            }

            // Layout children using the dedicated flex layout engine
            self.flex_layout_engine
                .layout_flex_children(node.clone(), &style, self);
        }
    }
}

/// Snapshot types safe to share across threads
#[derive(Clone)]
pub struct RenderNode {
    pub id: Id,
    pub bounds: Rect,
    pub style: Arc<Style>,
    pub text: Option<String>,
    pub children: Vec<RenderNode>,
}

pub fn build_render_tree(node: Rc<RefCell<Node>>) -> RenderNode {
    let nb = node.borrow();
    let mut children = Vec::with_capacity(nb.children.len());
    for c in &nb.children {
        children.push(build_render_tree(c.clone()));
    }
    RenderNode {
        id: nb.id,
        bounds: nb.layout.bounds,
        style: nb.layout.style.clone(),
        text: nb.text.clone(),
        children,
    }
}

#[cfg(test)]
mod flex_layout_flow_tests;

#[cfg(test)]
mod flex_layout_alignment_tests;

#[cfg(test)]
mod flex_layout_gap_tests;

#[cfg(test)]
mod flex_layout_grow_shrink_basis_tests;

#[cfg(test)]
mod margin_padding_tests;

#[cfg(test)]
mod hit_testing_tests;
