use crate::css_parser::parse_css;
use crate::layout::{build_render_snapshot, LayoutContext, RenderOp, RenderSnapshot};
use crate::Id;
use chrono::Local;
use std::fs;
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
    InputSetFocusedTextInput(Option<Id>),
    InputInsertText(String),
    InputDeleteBackward,
    InputMoveCaretLeft,
    InputMoveCaretRight,
    SetViewportSize(f64, f64),
    RemoveNode(Id),
    Scroll(Id, f64, f64),
    DumpDebugState,
    #[allow(unused)]
    Layout,
}

fn write_document_tree(
    node: &std::rc::Rc<std::cell::RefCell<crate::layout::Node>>,
    indent: usize,
    out: &mut String,
) {
    let node_borrow = node.borrow();
    let prefix = "  ".repeat(indent);
    out.push_str(&format!(
        "{prefix}- id={} text={:?} attrs={:?} bounds={:?} style={:#?}\n",
        node_borrow.id.value(),
        node_borrow.text,
        node_borrow.attributes,
        node_borrow.layout.bounds,
        node_borrow.layout.style,
    ));

    for child in &node_borrow.children {
        write_document_tree(child, indent + 1, out);
    }
}

fn write_render_tree(node: &crate::layout::RenderNode, indent: usize, out: &mut String) {
    let prefix = "  ".repeat(indent);
    out.push_str(&format!(
        "{prefix}- id={} text={:?} bounds={:?} style={:#?}\n",
        node.id.value(),
        node.text,
        node.bounds,
        node.style,
    ));

    for child in &node.children {
        write_render_tree(child, indent + 1, out);
    }
}

fn write_render_ops(ops: &[RenderOp], out: &mut String) {
    for (index, op) in ops.iter().enumerate() {
        match op {
            RenderOp::DrawBackgroundBorder(node) => {
                out.push_str(&format!(
                    "[{index}] DrawBackgroundBorder id={} bounds={:?}\n",
                    node.id.value(),
                    node.bounds,
                ));
            }
            RenderOp::DrawText(node) => {
                out.push_str(&format!(
                    "[{index}] DrawText id={} bounds={:?} text={:?}\n",
                    node.id.value(),
                    node.bounds,
                    node.text,
                ));
            }
            RenderOp::PushClip(rect) => {
                out.push_str(&format!("[{index}] PushClip rect={:?}\n", rect));
            }
            RenderOp::PopClip => {
                out.push_str(&format!("[{index}] PopClip\n"));
            }
        }
    }
}

fn dump_debug_state(
    ctx: &mut LayoutContext,
    snapshot: &Arc<RwLock<Option<RenderSnapshot>>>,
    message_sender: &WindowMessageSender,
) {
    ctx.layout();
    let root = ctx.document.root_node();
    let snap = build_render_snapshot(root.clone());
    *snapshot.write().unwrap() = Some(snap.clone());
    message_sender.send(WindowMessage::Redraw);

    let ts = Local::now().format("%Y-%m-%d_%H-%M-%S_%3f").to_string();
    let style_path = format!("style_{ts}.txt");
    let nodes_path = format!("nodes_{ts}.txt");

    let mut style_dump = String::new();
    style_dump.push_str(&format!("timestamp={ts}\n"));
    style_dump.push_str("\n[stylesheet]\n");
    style_dump.push_str(&format!("{:#?}\n", ctx.style_sheet));

    style_dump.push_str("\n[computed_styles_from_render_tree]\n");
    write_render_tree(&snap.root, 0, &mut style_dump);

    let mut nodes_dump = String::new();
    nodes_dump.push_str(&format!("timestamp={ts}\n"));
    nodes_dump.push_str("\n[document_tree]\n");
    write_document_tree(&root, 0, &mut nodes_dump);
    nodes_dump.push_str("\n[render_tree]\n");
    write_render_tree(&snap.root, 0, &mut nodes_dump);
    nodes_dump.push_str("\n[render_ops]\n");
    write_render_ops(&snap.render_list, &mut nodes_dump);

    let mut wrote_style = false;
    let mut wrote_nodes = false;

    match fs::write(&style_path, style_dump) {
        Ok(_) => wrote_style = true,
        Err(err) => eprintln!("Failed to write debug style dump {style_path}: {err}"),
    }

    match fs::write(&nodes_path, nodes_dump) {
        Ok(_) => wrote_nodes = true,
        Err(err) => eprintln!("Failed to write debug nodes dump {nodes_path}: {err}"),
    }

    if wrote_style && wrote_nodes {
        eprintln!("Debug dump written: {style_path}, {nodes_path}");
    }
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
                Command::InputSetFocusedTextInput(id) => {
                    if ctx.set_focused_text_input(id) {
                        let new_deadline = Instant::now() + Duration::from_millis(16);
                        deadline = Some(match deadline {
                            Some(existing) => existing.min(new_deadline),
                            None => new_deadline,
                        });
                    }
                }
                Command::InputInsertText(text) => {
                    if ctx.insert_text_at_focus(&text) {
                        let new_deadline = Instant::now() + Duration::from_millis(16);
                        deadline = Some(match deadline {
                            Some(existing) => existing.min(new_deadline),
                            None => new_deadline,
                        });
                    }
                }
                Command::InputDeleteBackward => {
                    if ctx.delete_backward_at_focus() {
                        let new_deadline = Instant::now() + Duration::from_millis(16);
                        deadline = Some(match deadline {
                            Some(existing) => existing.min(new_deadline),
                            None => new_deadline,
                        });
                    }
                }
                Command::InputMoveCaretLeft => {
                    if ctx.move_caret_left() {
                        let new_deadline = Instant::now() + Duration::from_millis(16);
                        deadline = Some(match deadline {
                            Some(existing) => existing.min(new_deadline),
                            None => new_deadline,
                        });
                    }
                }
                Command::InputMoveCaretRight => {
                    if ctx.move_caret_right() {
                        let new_deadline = Instant::now() + Duration::from_millis(16);
                        deadline = Some(match deadline {
                            Some(existing) => existing.min(new_deadline),
                            None => new_deadline,
                        });
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
                Command::RemoveNode(id) => {
                    ctx.remove_node(id);
                    if deadline.is_none() {
                        deadline = Some(Instant::now() + Duration::from_millis(100));
                    }
                }
                Command::Scroll(id, dx, dy) => {
                    let state = ctx.scroll_state.entry(id).or_default();
                    state.scroll_x.set(state.scroll_x.get() + dx);
                    state.scroll_y.set(state.scroll_y.get() + dy);
                    if deadline.is_none() {
                        deadline = Some(Instant::now() + Duration::from_millis(16));
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
                Command::DumpDebugState => {
                    dump_debug_state(&mut ctx, &snapshot, &message_sender);
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
