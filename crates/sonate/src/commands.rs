use crate::css_parser::parse_css;
use crate::layout::{build_render_snapshot, LayoutContext, RenderSnapshot};
use crate::Id;
use std::sync::{
    mpsc::{self, Receiver},
    Arc, RwLock,
};
use std::time::{Duration, Instant};

use crate::windowing::{WindowMessage, WindowMessageSender};

pub(crate) enum Command {
    AddStylesheet(String),
    CreateNode(Id, Option<String>),
    SetParent(Id, Id),
    SetAttribute(Id, String, String),
    SetViewportSize(f64, f64),
    #[allow(unused)]
    Layout,
}

pub(crate) fn handle_commands(
    rx: Receiver<Command>,
    snapshot: Arc<RwLock<Option<RenderSnapshot>>>,
    message_sender: WindowMessageSender,
) {
    let mut ctx = LayoutContext::new();
    let mut deadline: Option<Instant> = None;

    loop {
        // Determine timeout based on debounce deadline
        let timeout = match deadline {
            Some(dl) => {
                let now = Instant::now();
                if dl <= now {
                    // Deadline expired: run layout now
                    ctx.layout();
                    let root = ctx.document.root_node();
                    let snap = build_render_snapshot(root);
                    *snapshot.write().unwrap() = Some(snap);
                    message_sender.send(WindowMessage::Redraw);
                    deadline = None;
                    // After layout, continue to next iteration
                    continue;
                } else {
                    dl - now
                }
            }
            None => Duration::from_millis(u64::MAX / 2), // effectively wait forever
        };

        match rx.recv_timeout(timeout) {
            Ok(cmd) => match cmd {
                Command::AddStylesheet(css) => match parse_css(&css) {
                    Ok(sheet) => {
                        for rule in sheet.rules {
                            ctx.style_sheet.add_rule(rule);
                        }
                        if deadline.is_none() {
                            deadline = Some(Instant::now() + Duration::from_millis(100));
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to parse CSS: {}", e);
                    }
                },
                Command::CreateNode(id, text) => {
                    ctx.document.create_node(id, text);
                    if deadline.is_none() {
                        deadline = Some(Instant::now() + Duration::from_millis(100));
                    }
                }
                Command::SetParent(p, c) => {
                    ctx.document.set_parent(p, c).expect("data thread down");
                    if deadline.is_none() {
                        deadline = Some(Instant::now() + Duration::from_millis(100));
                    }
                }
                Command::SetAttribute(id, k, v) => {
                    ctx.document.set_attribute(id, k, v);
                    if deadline.is_none() {
                        deadline = Some(Instant::now() + Duration::from_millis(100));
                    }
                }
                Command::SetViewportSize(width, height) => {
                    if width > 0.0 && height > 0.0 {
                        ctx.set_viewport_size(width, height);

                        // Keep resize responsive without relayouting on every single event.
                        let new_deadline = Instant::now() + Duration::from_millis(16);
                        deadline = Some(match deadline {
                            Some(existing) => existing.min(new_deadline),
                            None => new_deadline,
                        });
                    }
                }
                Command::Layout => {
                    // Immediate layout flush
                    ctx.layout();
                    let root = ctx.document.root_node();
                    let snap = build_render_snapshot(root);
                    *snapshot.write().unwrap() = Some(snap);
                    message_sender.send(WindowMessage::Redraw);
                    deadline = None;
                }
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // handled at top loop when checking expired deadline
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}
