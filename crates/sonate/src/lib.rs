mod backend;
mod commands;
mod css_parser;
mod flex_layout;
mod layout;
mod painter;
mod scrollbar;
mod scrollbar_interaction;
mod style;
mod style_matching;
mod text;
mod windowing;

use crate::backend::InputKey;
use commands::Command;
use layout::RenderSnapshot;
use painter::Painter;
use scrollbar::ScrollbarAxis;
use scrollbar_interaction::{begin_scrollbar_drag, update_scrollbar_drag, ActiveScrollbarDrag};
use std::sync::Mutex;
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, RwLock,
};
use std::thread;

use crate::windowing::WindowMessageSender;

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
    snapshot: Arc<RwLock<Option<RenderSnapshot>>>,
    root_id: Id,
    running: Arc<Mutex<()>>,
    message_sender: WindowMessageSender,
}

#[derive(Default)]
pub struct Params {
    pub on_click: Option<Box<dyn Fn(f64, f64, Vec<Id>)>>,
    pub on_scroll: Option<Box<dyn Fn(Id, f64, f64)>>,
}

#[derive(Debug)]
pub enum Error {
    AlreadyRunning,
    UnknownError(String),
}

fn debug_enabled() -> bool {
    std::env::var("SONATE_DEBUG")
        .ok()
        .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
}

impl Engine {
    /// Create a new CSS engine instance
    pub fn new() -> Self {
        let (tx, rx): (Sender<Command>, Receiver<Command>) = channel();
        let snapshot: Arc<RwLock<Option<RenderSnapshot>>> = Arc::new(RwLock::new(None));
        let snapshot_for_thread = Arc::clone(&snapshot);
        let message_sender = WindowMessageSender::new();
        let message_sender_for_thread = message_sender.clone();

        // Spawn thread to handle the commands without blocking the main thread
        thread::spawn(move || {
            commands::handle_commands(rx, snapshot_for_thread, message_sender_for_thread)
        });

        Self {
            sender: tx,
            snapshot,
            root_id: Id::from_u64(0),
            running: Arc::new(Mutex::new(())),
            message_sender,
        }
    }

    // Run the event loop
    pub fn run(&self, params: Params) -> Result<(), Error> {
        // only allow running once
        let _lock = self.running.try_lock().map_err(|_| Error::AlreadyRunning)?;
        let debug_logging = debug_enabled();

        let this1 = self.clone();
        let this2 = self.clone();
        let this3 = self.clone();
        let focus_sender = self.sender.clone();
        let scroll_drag_sender = self.sender.clone();
        let input_sender = self.sender.clone();
        let key_sender = self.sender.clone();
        let resize_sender = self.sender.clone();
        let debug_dump_sender = self.sender.clone();
        let drag_state: Arc<Mutex<Option<ActiveScrollbarDrag>>> = Arc::new(Mutex::new(None));
        let drag_state_down = drag_state.clone();
        let drag_state_move = drag_state.clone();
        let drag_state_up = drag_state.clone();
        let this4 = self.clone();
        let this5 = self.clone();

        let mut params = windowing::Params {
            on_draw: Box::new(move |canvas| {
                if let Some(snapshot) = this1.get_current_snapshot() {
                    let mut painter = Painter::new(canvas);
                    painter.paint(&snapshot);
                }
            }),
            on_click: Box::new(move |x, y| {
                if let Some(snapshot) = this2.get_current_snapshot() {
                    let elements = snapshot.root.find_element_at_position(x, y);
                    if debug_logging {
                        let ids: Vec<u64> = elements.iter().map(|id| id.as_u64()).collect();
                        let chain: Vec<String> = elements
                            .iter()
                            .filter_map(|id| {
                                snapshot.root.find_node_by_id(*id).map(|node| {
                                    format!(
                                        "id={} bounds=({}, {}, {}, {}) text={:?}",
                                        id.as_u64(),
                                        node.bounds.x,
                                        node.bounds.y,
                                        node.bounds.width,
                                        node.bounds.height,
                                        node.text
                                    )
                                })
                            })
                            .collect();
                        println!(
                            "[sonate] click x={} y={} hit_ids={:?} hit_chain={:?}",
                            x, y, ids, chain
                        );
                    }

                    let focused_text_input = elements.iter().find_map(|id| {
                        snapshot.root.find_node_by_id(*id).and_then(|node| {
                            node.style
                                .widget
                                .is_some_and(|widget| widget.is_text_input())
                                .then_some(*id)
                        })
                    });
                    let _ =
                        focus_sender.send(Command::InputSetFocusedTextInput(focused_text_input));

                    if let Some(ref on_click) = params.on_click {
                        on_click(x, y, elements);
                    }
                }
            }),
            on_mouse_down: Box::new(move |x, y| {
                if let Some(snapshot) = this4.get_current_snapshot() {
                    let active = begin_scrollbar_drag(&snapshot, x, y);
                    let consumed = active.is_some();
                    *drag_state_down.lock().unwrap() = active;
                    return consumed;
                }

                *drag_state_down.lock().unwrap() = None;
                false
            }),
            on_mouse_move: Box::new(move |x, y| {
                let active = *drag_state_move.lock().unwrap();
                let Some(active) = active else {
                    return;
                };

                let Some(snapshot) = this5.get_current_snapshot() else {
                    *drag_state_move.lock().unwrap() = None;
                    return;
                };

                let Some(update) = update_scrollbar_drag(&snapshot, active, x, y) else {
                    *drag_state_move.lock().unwrap() = None;
                    return;
                };

                match update.axis {
                    ScrollbarAxis::Vertical => {
                        let _ = scroll_drag_sender
                            .send(Command::SetScrollY(update.container_id, update.new_scroll));
                    }
                    ScrollbarAxis::Horizontal => {
                        let _ = scroll_drag_sender
                            .send(Command::SetScrollX(update.container_id, update.new_scroll));
                    }
                }
            }),
            on_mouse_up: Box::new(move |_x, _y| {
                *drag_state_up.lock().unwrap() = None;
            }),
            on_resize: Box::new(move |width, height| {
                let _ = resize_sender.send(Command::SetViewportSize(width, height));
            }),
            on_scroll: Box::new(move |x, y, dx, dy| {
                if let Some(snapshot) = this3.get_current_snapshot() {
                    if let Some(scrollable_id) = snapshot.root.find_scrollable_at_position(x, y) {
                        if debug_logging {
                            println!(
                                "[sonate] scroll x={} y={} dx={} dy={} target_id={}",
                                x,
                                y,
                                dx,
                                dy,
                                scrollable_id.as_u64()
                            );
                        }
                        this3.scroll(scrollable_id, -dx, -dy);
                    } else if debug_logging {
                        println!(
                            "[sonate] scroll x={} y={} dx={} dy={} target_id=<none>",
                            x, y, dx, dy
                        );
                    }
                }
            }),
            on_text_input: Box::new(move |text| {
                let _ = input_sender.send(Command::InputInsertText(text));
            }),
            on_key: Box::new(move |key| {
                let command = match key {
                    InputKey::ArrowLeft => Command::InputMoveCaretLeft,
                    InputKey::ArrowRight => Command::InputMoveCaretRight,
                    InputKey::Backspace => Command::InputDeleteBackward,
                };
                let _ = key_sender.send(command);
            }),
            on_debug_dump: Box::new(move || {
                let _ = debug_dump_sender.send(Command::DumpDebugState);
            }),
        };

        windowing::run(&mut params, self.message_sender.clone())
            .map_err(|err| Error::UnknownError(err.to_string()))?;

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

    /// Remove a node and all its descendants from the document
    pub fn remove_node(&self, node_id: Id) {
        self.sender
            .send(Command::RemoveNode(node_id))
            .expect("data thread down");
    }

    /// Scroll a node by the given delta
    pub fn scroll(&self, node_id: Id, dx: f64, dy: f64) {
        self.sender
            .send(Command::Scroll(node_id, dx, dy))
            .expect("data thread down");
    }

    /// Get the root node ID of the document
    pub fn root_id(&self) -> Id {
        self.root_id
    }

    /// Get a cloned copy of the current render snapshot for drawing
    fn get_current_snapshot(&self) -> Option<RenderSnapshot> {
        self.snapshot.read().unwrap().as_ref().cloned()
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}
