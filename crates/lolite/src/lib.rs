mod backend;
mod commands;
mod css_parser;
mod flex_layout;
mod layout;
mod painter;
mod style;
mod windowing;

#[cfg(test)]
mod css_parser_tests;

use commands::Command;
use layout::RenderNode;
use painter::Painter;
use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, RwLock,
};
use std::thread;
use windowing::{run, Params};

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

#[derive(Clone)]
pub struct Engine {
    sender: Sender<Command>,
    snapshot: Arc<RwLock<Option<RenderNode>>>,
    root_id: Id,
    next_id: Arc<AtomicU64>,
    running: Arc<Mutex<()>>,
}

#[derive(Debug)]
pub enum RunError {
    AlreadyRunning,
    UnknownError(String),
}

impl Engine {
    /// Create a new CSS engine instance
    pub fn new() -> Self {
        let (tx, rx): (Sender<Command>, Receiver<Command>) = channel();
        let snapshot: Arc<RwLock<Option<RenderNode>>> = Arc::new(RwLock::new(None));
        let snapshot_for_thread = Arc::clone(&snapshot);

        // Spawn thread to handle the commands without blocking the main thread
        thread::spawn(move || commands::handle_commands(rx, snapshot_for_thread));

        Self {
            sender: tx,
            snapshot,
            root_id: Id::from_u64(0),
            // 0 is reserved for root
            next_id: Arc::new(AtomicU64::new(1)),
            running: Arc::new(Mutex::new(())),
        }
    }

    // Run the event loop
    pub fn run(&self) -> Result<(), RunError> {
        // only allow running once
        let _lock = self
            .running
            .try_lock()
            .map_err(|_| RunError::AlreadyRunning)?;

        let this = self.clone();

        let params = Params {
            on_draw: Box::new(move |canvas| {
                if let Some(snapshot) = this.get_current_snapshot() {
                    let mut painter = Painter::new(canvas);
                    painter.paint(&snapshot);
                }
            }),
            on_click: Some(Box::new(move |_x, _y| {
                // Perform hit testing
                // let elements = engine_for_click.find_element_at_position(x, y); // here we should already know which elements we clicked on

                // if elements.is_empty() {
                //     println!("Click detected on background at ({:.1}, {:.1})", x, y);
                // } else {
                //     println!(
                //         "Click detected at ({:.1}, {:.1}) on {} elements:",
                //         x,
                //         y,
                //         elements.len()
                //     );
                //     for (i, element_id) in elements.iter().enumerate() {
                //         println!("  Level {}: Element ID {:?}", i, element_id.value());
                //     }
                // }
            })),
        };

        run(&RefCell::new(params)).map_err(|err| RunError::UnknownError(err.to_string()))?;

        Ok(())
    }

    /// Add a CSS stylesheet
    pub fn add_stylesheet(&self, css_content: &str) {
        let _ = self
            .sender
            .send(Command::AddStylesheet(css_content.to_string()))
            .expect("data thread down");
    }

    /// Create a new document node with optional text content
    pub fn create_node(&self, text: Option<String>) -> Id {
        // Generate unique id locally without waiting on the data thread
        let id_value = self.next_id.fetch_add(1, Ordering::Relaxed);
        let id = Id::from_u64(id_value);
        self.sender
            .send(Command::CreateNode(id, text))
            .expect("data thread down");
        id
    }

    /// Set a parent-child relationship between nodes
    pub fn set_parent(&self, parent_id: Id, child_id: Id) {
        self.sender
            .send(Command::SetParent(parent_id, child_id))
            .expect("data thread down");
    }

    /// Set an attribute on a node
    pub fn set_attribute(&self, node_id: Id, key: String, value: String) {
        self.sender
            .send(Command::SetAttribute(node_id, key, value))
            .expect("data thread down");
    }

    /// Get the root node ID of the document
    pub fn root_id(&self) -> Id {
        self.root_id
    }

    // /// Find elements at a specific position (for hit testing)
    // pub fn find_element_at_position(&self, x: f64, y: f64) -> Vec<Id> {
    //     if let Some(snapshot) = self.snapshot.read().unwrap().as_ref() {
    //         self.find_element_at_position_recursive(snapshot, snapshot, x, y)
    //     } else {
    //         // No snapshot available yet (layout not run)
    //         vec![]
    //     }
    // }

    // /// Recursively find elements at a specific position in the render tree
    // fn find_element_at_position_recursive(
    //     &self,
    //     root: &RenderNode,
    //     node: &RenderNode,
    //     x: f64,
    //     y: f64,
    // ) -> Vec<Id> {
    //     let mut result = Vec::new();

    //     // Check if the point is within this node's bounds
    //     if !self.point_in_bounds(&node.bounds, x, y) {
    //         return result;
    //     }

    //     // Check children in reverse order (later children are rendered on top)
    //     for child in node.children.iter().rev() {
    //         let child_result = self.find_element_at_position_recursive(root, child, x, y);
    //         if !child_result.is_empty() {
    //             // Found a hit in a child, return the child's result chain
    //             return child_result;
    //         }
    //     }

    //     // No child contains the point, so this node is the topmost
    //     // Build the parent chain by traversing up from this node
    //     result.push(node.id);

    //     // Since RenderNode doesn't have parent pointers, we need to build the chain
    //     // by finding this node's ancestors in the tree
    //     self.build_parent_chain(root, node.id, &mut result);

    //     result
    // }

    // /// Build the parent chain for a given node ID by traversing the render tree
    // fn build_parent_chain(&self, root: &RenderNode, target_id: Id, result: &mut Vec<Id>) {
    //     self.find_parent_recursive(root, target_id, result);
    // }

    // /// Recursively find the parent chain for a target node
    // fn find_parent_recursive(
    //     &self,
    //     node: &RenderNode,
    //     target_id: Id,
    //     result: &mut Vec<Id>,
    // ) -> bool {
    //     // Check if any direct child is our target
    //     for child in &node.children {
    //         if child.id == target_id {
    //             // Found the target as a direct child, add this node as parent
    //             result.push(node.id);
    //             return true;
    //         }
    //     }

    //     // Check if target is in any child subtree
    //     for child in &node.children {
    //         if self.find_parent_recursive(child, target_id, result) {
    //             // Target was found in this child's subtree, add this node as ancestor
    //             result.push(node.id);
    //             return true;
    //         }
    //     }

    //     false
    // }

    // /// Check if a point (x, y) is within the given bounds
    // fn point_in_bounds(&self, bounds: &engine::Rect, x: f64, y: f64) -> bool {
    //     x >= bounds.x
    //         && x <= bounds.x + bounds.width
    //         && y >= bounds.y
    //         && y <= bounds.y + bounds.height
    // }

    /// Get a cloned copy of the current render snapshot for drawing
    fn get_current_snapshot(&self) -> Option<RenderNode> {
        self.snapshot.read().unwrap().as_ref().cloned()
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}
