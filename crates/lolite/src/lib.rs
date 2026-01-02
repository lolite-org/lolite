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
use std::sync::Mutex;
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, RwLock,
};
use std::thread;

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
    running: Arc<Mutex<()>>,
}

pub struct Params {
    pub on_click: Option<Box<dyn Fn(f64, f64, Vec<Id>)>>,
}

#[derive(Debug)]
pub enum Error {
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
            running: Arc::new(Mutex::new(())),
        }
    }

    // Run the event loop
    pub fn run(&self, params: Params) -> Result<(), Error> {
        // only allow running once
        let _lock = self.running.try_lock().map_err(|_| Error::AlreadyRunning)?;

        let this1 = self.clone();
        let this2 = self.clone();

        let mut params = windowing::Params {
            on_draw: Box::new(move |canvas| {
                if let Some(snapshot) = this1.get_current_snapshot() {
                    let mut painter = Painter::new(canvas);
                    painter.paint(&snapshot);
                }
            }),
            on_click: Box::new(move |x, y| {
                if let Some(snapshot) = this2.get_current_snapshot() {
                    let elements = snapshot.find_element_at_position(x, y);

                    if let Some(ref on_click) = params.on_click {
                        on_click(x, y, elements);
                    }
                }
            }),
        };

        windowing::run(&mut params).map_err(|err| Error::UnknownError(err.to_string()))?;

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
    pub fn create_node(&self, id: Id, text: Option<String>) -> Id {
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
