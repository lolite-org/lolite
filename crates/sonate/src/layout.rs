use crate::{
    flex_layout::FlexLayoutEngine,
    style::{BoxSizing, Length, Overflow, Style, StyleSheet, ZIndex},
    text::{default_text_measurer, FontSpec, TextMeasurer},
    Id,
};

// Many layout tests do `use super::*;` and refer to `Selector::...`.
// Keep this import in the module namespace for those tests.
#[allow(unused_imports)]
use crate::style::Selector;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::Arc,
};

#[derive(Default, Debug)]
pub struct ScrollState {
    pub scroll_x: Cell<f64>,
    pub scroll_y: Cell<f64>,
}

impl ScrollState {
    #[allow(unused)]
    pub fn new(scroll_x: f64, scroll_y: f64) -> Self {
        Self {
            scroll_x: Cell::new(scroll_x),
            scroll_y: Cell::new(scroll_y),
        }
    }
}

#[derive(Default)]
pub struct Layout {
    pub bounds: Rect,
    pub style: Arc<Style>,
}

#[derive(Clone, Debug, Default)]
pub struct TextInputState {
    pub focused: bool,
    pub caret: usize,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    #[allow(unused)]
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    #[allow(unused)]
    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }

    pub fn intersect(&self, other: &Rect) -> Option<Rect> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width).min(other.x + other.width);
        let y2 = (self.y + self.height).min(other.y + other.height);

        let w = x2 - x1;
        let h = y2 - y1;
        if w <= 0.0 || h <= 0.0 {
            return None;
        }

        Some(Rect {
            x: x1,
            y: y1,
            width: w,
            height: h,
        })
    }
}

#[derive(Default)]
#[allow(unused)]
pub struct Node {
    pub id: Id,
    pub text: Option<String>,
    pub text_input_state: Option<TextInputState>,
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

    pub fn is_text_node(&self) -> bool {
        self.text.is_some()
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

    pub fn remove_node(&mut self, id: Id) -> Vec<Id> {
        let node = match self.nodes.get(&id) {
            Some(n) => n.clone(),
            None => return Vec::new(),
        };

        // Detach from parent
        if let Some(parent_id) = node.borrow().parent {
            if let Some(parent) = self.nodes.get(&parent_id) {
                parent.borrow_mut().children.retain(|c| c.borrow().id != id);
            }
        }

        // Collect all descendant ids (including this node)
        let mut removed = Vec::new();
        Self::collect_ids(&node, &mut removed);

        for rid in &removed {
            self.nodes.remove(rid);
        }

        removed
    }

    fn collect_ids(node: &Rc<RefCell<Node>>, out: &mut Vec<Id>) {
        let borrow = node.borrow();
        out.push(borrow.id);
        for child in &borrow.children {
            Self::collect_ids(child, out);
        }
    }
}

pub struct LayoutContext {
    pub document: Document,
    pub style_sheet: StyleSheet,
    flex_layout_engine: FlexLayoutEngine,
    pub text_measurer: Arc<dyn TextMeasurer>,
    viewport_size: Size,
    focused_text_input: Option<Id>,
    pub scroll_state: HashMap<Id, ScrollState>,
}

impl LayoutContext {
    pub fn new() -> Self {
        Self {
            document: Document::new(),
            style_sheet: StyleSheet::new(),
            flex_layout_engine: FlexLayoutEngine::new(),
            text_measurer: default_text_measurer(),
            viewport_size: Size {
                width: 800.0,
                height: 500.0,
            },
            focused_text_input: None,
            scroll_state: HashMap::new(),
        }
    }

    pub fn set_viewport_size(&mut self, width: f64, height: f64) {
        self.viewport_size = Size { width, height };
    }

    pub fn layout(&mut self) {
        self.text_measurer.begin_layout_pass();
        self.layout_node(self.document.root.clone(), 0.0, 0.0);
        self.text_measurer.end_layout_pass_and_sweep();
    }

    /// Remove a node from the document and clean up associated scroll state.
    pub fn remove_node(&mut self, id: Id) {
        let removed = self.document.remove_node(id);
        if self
            .focused_text_input
            .is_some_and(|focused| removed.contains(&focused))
        {
            self.focused_text_input = None;
        }
        for rid in &removed {
            self.scroll_state.remove(rid);
        }
    }

    pub fn set_focused_text_input(&mut self, id: Option<Id>) -> bool {
        if self.focused_text_input == id {
            return false;
        }

        if let Some(previous) = self.focused_text_input.take() {
            if let Some(node) = self.document.get_node(previous) {
                if let Some(state) = node.borrow_mut().text_input_state.as_mut() {
                    state.focused = false;
                }
            }
        }

        if let Some(target) = id {
            let Some(node) = self.document.get_node(target) else {
                return true;
            };

            let mut node = node.borrow_mut();
            Self::prepare_text_input_node(&mut node);
            let text_len = node.text.as_deref().map(char_len).unwrap_or(0);
            if let Some(state) = node.text_input_state.as_mut() {
                state.focused = true;
                state.caret = text_len;
            }
            self.focused_text_input = Some(target);
        }

        true
    }

    pub fn insert_text_at_focus(&mut self, text: &str) -> bool {
        let Some(node) = self
            .focused_text_input
            .and_then(|id| self.document.get_node(id))
        else {
            return false;
        };

        let mut node = node.borrow_mut();
        Self::prepare_text_input_node(&mut node);
        let caret = node
            .text_input_state
            .as_ref()
            .expect("text input state must exist")
            .caret
            .min(node.text.as_deref().map(char_len).unwrap_or(0));
        {
            let current = node.text.get_or_insert_with(String::new);
            let byte_index = byte_index_for_char(current, caret);
            current.insert_str(byte_index, text);
        }
        let updated_text = node.text.clone().unwrap_or_default();
        node.text_input_state
            .as_mut()
            .expect("text input state must exist")
            .caret = caret + char_len(text);
        node.attributes.insert("value".to_string(), updated_text);
        true
    }

    pub fn delete_backward_at_focus(&mut self) -> bool {
        let Some(node) = self
            .focused_text_input
            .and_then(|id| self.document.get_node(id))
        else {
            return false;
        };

        let mut node = node.borrow_mut();
        Self::prepare_text_input_node(&mut node);
        let caret = node
            .text_input_state
            .as_ref()
            .expect("text input state must exist")
            .caret
            .min(node.text.as_deref().map(char_len).unwrap_or(0));
        if caret == 0 {
            return false;
        }

        {
            let current = node.text.get_or_insert_with(String::new);
            let start = byte_index_for_char(current, caret - 1);
            let end = byte_index_for_char(current, caret);
            current.replace_range(start..end, "");
        }
        let updated_text = node.text.clone().unwrap_or_default();
        node.text_input_state
            .as_mut()
            .expect("text input state must exist")
            .caret = caret - 1;
        node.attributes.insert("value".to_string(), updated_text);
        true
    }

    pub fn move_caret_left(&mut self) -> bool {
        self.move_caret(-1)
    }

    pub fn move_caret_right(&mut self) -> bool {
        self.move_caret(1)
    }

    fn move_caret(&mut self, delta: isize) -> bool {
        let Some(node) = self
            .focused_text_input
            .and_then(|id| self.document.get_node(id))
        else {
            return false;
        };

        let mut node = node.borrow_mut();
        Self::prepare_text_input_node(&mut node);
        let text_len = node.text.as_deref().map(char_len).unwrap_or(0);
        let state = node
            .text_input_state
            .as_mut()
            .expect("text input state must exist");
        let next = if delta.is_negative() {
            state.caret.saturating_sub(delta.unsigned_abs())
        } else {
            state.caret.saturating_add(delta as usize).min(text_len)
        };
        if next == state.caret {
            return false;
        }
        state.caret = next;
        true
    }

    pub(crate) fn prepare_text_input_node(node: &mut Node) {
        let initial_text = node
            .text
            .clone()
            .or_else(|| node.attributes.get("value").cloned())
            .unwrap_or_default();
        node.text = Some(initial_text.clone());

        let caret = char_len(&initial_text);
        node.text_input_state.get_or_insert_with(|| TextInputState {
            focused: false,
            caret,
        });
    }

    pub fn layout_node(&self, node: Rc<RefCell<Node>>, x: f64, y: f64) {
        // Get style for this node - merge existing style with CSS rules
        let style = {
            let node_borrow = node.borrow();
            // Start with existing style as base (this preserves manually set properties like flex_wrap)
            let mut style = node_borrow.layout.style.as_ref().clone();

            crate::style_matching::apply_matching_rules(
                &mut style,
                &node_borrow.attributes,
                &self.style_sheet,
            );
            style
        };

        // Set position (margins will be applied by flex layout engine for flex items)
        {
            let mut node_borrow = node.borrow_mut();
            node_borrow.layout.bounds.x = x;
            node_borrow.layout.bounds.y = y;
        }

        let is_text_input = style.widget.is_some_and(|widget| widget.is_text_input());
        if is_text_input {
            Self::prepare_text_input_node(&mut node.borrow_mut());
        }

        let is_leaf = node.borrow().children.is_empty();
        let is_text_node = node.borrow().is_text_node() || is_text_input;

        // Sonate stores `layout.bounds` as the element's border-box.
        // `box-sizing` determines whether CSS `width/height` refer to the content-box or border-box.
        let resolved_box_sizing = style.box_sizing.unwrap_or(BoxSizing::ContentBox);
        let padding = style.padding.resolved();
        let padding_w = padding.left.to_px() + padding.right.to_px();
        let padding_h = padding.top.to_px() + padding.bottom.to_px();
        let border = style.border_width.resolved();
        let border_w = border.left.to_px() + border.right.to_px();
        let border_h = border.top.to_px() + border.bottom.to_px();

        let resolve_border_box =
            |specified: Option<Length>, fallback: f64, padding_sum: f64, border_sum: f64| -> f64 {
                let Some(Length::Px(px)) = specified else {
                    return fallback;
                };

                match resolved_box_sizing {
                    BoxSizing::ContentBox => px + padding_sum + border_sum,
                    BoxSizing::BorderBox => px,
                }
            };

        if is_leaf {
            // Leaf node - use specified dimensions or defaults.
            // If this is a text node, prefer intrinsic text sizing.
            let mut fallback_width_border_box = 100.0;
            let mut fallback_height_border_box = 30.0;

            if is_text_node {
                if let Some(text) = node.borrow().text.as_deref() {
                    let font = FontSpec::from_style(&style);

                    // Width: if not specified, use unwrapped intrinsic width.
                    if matches!(style.width, Some(Length::Auto)) {
                        let text_size = self.text_measurer.measure_unwrapped(text, &font);
                        fallback_width_border_box = text_size.width + padding_w + border_w;
                    }

                    // Height: if not specified, try to wrap to a specified width (if any), else unwrapped.
                    if matches!(style.height, Some(Length::Auto)) {
                        let text_size = match style.width {
                            Some(Length::Px(specified_width_px)) if specified_width_px > 0.0 => {
                                // Wrap within the content box width.
                                let content_max_width = match resolved_box_sizing {
                                    BoxSizing::ContentBox => specified_width_px,
                                    BoxSizing::BorderBox => {
                                        (specified_width_px - padding_w - border_w).max(0.0)
                                    }
                                };
                                self.text_measurer
                                    .measure_wrapped(text, &font, content_max_width)
                            }
                            _ => self.text_measurer.measure_unwrapped(text, &font),
                        };

                        fallback_height_border_box = text_size.height + padding_h + border_h;
                    }
                }
            }

            let mut node_borrow = node.borrow_mut();
            node_borrow.layout.bounds.width =
                resolve_border_box(style.width, fallback_width_border_box, padding_w, border_w);
            node_borrow.layout.bounds.height = resolve_border_box(
                style.height,
                fallback_height_border_box,
                padding_h,
                border_h,
            );
            node_borrow.layout.style = Arc::new(style);
        } else {
            // Container node - handle flexbox layout
            let is_root = node.borrow().parent.is_none();
            let fallback_width = if is_root {
                self.viewport_size.width
            } else {
                800.0
            };
            let fallback_height = if is_root {
                self.viewport_size.height
            } else {
                500.0
            };

            let container_width =
                resolve_border_box(style.width, fallback_width, padding_w, border_w);
            let container_height =
                resolve_border_box(style.height, fallback_height, padding_h, border_h);

            // Set container dimensions
            {
                let mut node_borrow = node.borrow_mut();
                node_borrow.layout.bounds.width = container_width;
                node_borrow.layout.bounds.height = container_height;
                node_borrow.layout.style = Arc::new(style.clone());
            }

            // Layout children using the dedicated flex layout engine.
            // Scroll offset for overflow: scroll/auto is applied inside
            // layout_flex_children after children have been positioned.
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
    pub text_input_state: Option<TextInputState>,
    pub children: Vec<RenderNode>,
}

#[derive(Clone)]
pub enum RenderOp {
    DrawBackgroundBorder(RenderNode),
    DrawText(RenderNode),
    PushClip(Rect),
    PopClip,
}

/// Snapshot types safe to share across threads.
///
/// Keeps the tree (useful for hit testing + inspection) plus a flat render list
/// for painting.
#[derive(Clone)]
pub struct RenderSnapshot {
    pub root: RenderNode,
    pub render_list: Vec<RenderOp>,
}

/// Intermediate representation that groups render output into stacking contexts.
///
/// This is intentionally built *alongside* the `RenderNode` tree so we can later
/// compile it into a single paint-order list (and eventually sort entries by z-index).
#[derive(Clone)]
pub struct StackingContext {
    /// The draw item that owns this context.
    ///
    /// `None` means this is the synthetic root context.
    pub owner_node: Option<RenderNode>,

    /// Child entries in this context.
    pub entries: Vec<StackingEntry>,
}

#[derive(Clone)]
pub enum StackingEntry {
    Context(StackingContext),
}

impl Default for StackingContext {
    fn default() -> Self {
        Self {
            owner_node: None,
            entries: Vec::new(),
        }
    }
}

impl StackingContext {
    fn z_index_of_owner(&self) -> i32 {
        let Some(owner) = &self.owner_node else {
            return 0;
        };

        match owner.style.z_index {
            Some(ZIndex::Index(value)) => value,
            Some(ZIndex::Auto) | None => 0,
        }
    }

    pub fn sort_by_z_index(&mut self) {
        for entry in &mut self.entries {
            match entry {
                StackingEntry::Context(ctx) => ctx.sort_by_z_index(),
            }
        }

        // Stable sort: preserves source order for equal z-index values.
        self.entries.sort_by_key(|entry| match entry {
            StackingEntry::Context(ctx) => ctx.z_index_of_owner(),
        });
    }

    pub fn to_render_list(&self) -> Vec<RenderOp> {
        let mut out = Vec::new();
        self.write_render_ops(&mut out);
        out
    }

    fn write_render_ops(&self, out: &mut Vec<RenderOp>) {
        if let Some(owner) = &self.owner_node {
            out.push(RenderOp::DrawBackgroundBorder(RenderNode {
                id: owner.id,
                bounds: owner.bounds,
                style: owner.style.clone(),
                text: owner.text.clone(),
                text_input_state: owner.text_input_state.clone(),
                children: Vec::new(),
            }));

            let clips_overflow = matches!(
                owner.style.overflow,
                Some(Overflow::Hidden) | Some(Overflow::Scroll) | Some(Overflow::Auto)
            );
            if clips_overflow {
                let border = owner.style.border_width.resolved();
                let left = border.left.to_px();
                let top = border.top.to_px();
                let right = border.right.to_px();
                let bottom = border.bottom.to_px();

                let clip_rect = Rect {
                    x: owner.bounds.x + left,
                    y: owner.bounds.y + top,
                    width: (owner.bounds.width - left - right).max(0.0),
                    height: (owner.bounds.height - top - bottom).max(0.0),
                };
                out.push(RenderOp::PushClip(clip_rect));
            }

            out.push(RenderOp::DrawText(RenderNode {
                id: owner.id,
                bounds: owner.bounds,
                style: owner.style.clone(),
                text: owner.text.clone(),
                text_input_state: owner.text_input_state.clone(),
                children: Vec::new(),
            }));

            for entry in &self.entries {
                match entry {
                    StackingEntry::Context(ctx) => ctx.write_render_ops(out),
                }
            }

            if clips_overflow {
                out.push(RenderOp::PopClip);
            }
            return;
        }

        // Synthetic root context only forwards to child entries.
        for entry in &self.entries {
            match entry {
                StackingEntry::Context(ctx) => ctx.write_render_ops(out),
            }
        }
    }
}

impl RenderNode {
    /// Find the element at the given position (x, y).
    ///
    /// Returns a `Vec<Id>` where the first element is the topmost element at the position,
    /// and subsequent elements are its parents up to the root.
    /// This enables event bubbling by providing the full parent chain.
    pub fn find_element_at_position(&self, x: f64, y: f64) -> Vec<Id> {
        self.find_path_at_position(x, y, None).unwrap_or_default()
    }

    /// Find the nearest scrollable ancestor at the given position.
    /// Returns the Id of the first element in the hit-test chain that has
    /// `overflow: scroll` or `overflow: auto`.
    pub fn find_scrollable_at_position(&self, x: f64, y: f64) -> Option<Id> {
        let path = self.find_element_at_position(x, y);
        // path is [target, parent, grandparent, ...]
        // Walk the path and find the first node with overflow: scroll/auto
        for id in &path {
            if let Some(node) = self.find_node_by_id(*id) {
                match node.style.overflow {
                    Some(Overflow::Scroll) | Some(Overflow::Auto) => return Some(*id),
                    _ => {}
                }
            }
        }
        None
    }

    /// Find a RenderNode by its Id in the tree.
    pub fn find_node_by_id(&self, target: Id) -> Option<&RenderNode> {
        if self.id == target {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find_node_by_id(target) {
                return Some(found);
            }
        }
        None
    }

    fn find_path_at_position(&self, x: f64, y: f64, clip: Option<Rect>) -> Option<Vec<Id>> {
        // The inherited clip applies to this node itself (it is part of the parent's contents).
        if let Some(clip) = &clip {
            if !clip.contains_point(x, y) {
                return None;
            }
        }

        if !self.bounds.contains_point(x, y) {
            return None;
        }

        // A node's overflow clip applies to its *descendants* (content), not to its own border box.
        let descendant_clip = match self.style.overflow {
            Some(Overflow::Hidden) | Some(Overflow::Scroll) | Some(Overflow::Auto) => {
                // Clip to padding-box (border-box inset by border widths).
                let border = self.style.border_width.resolved();
                let border_left = border.left.to_px();
                let border_top = border.top.to_px();
                let border_right = border.right.to_px();
                let border_bottom = border.bottom.to_px();

                let padding_box = Rect {
                    x: self.bounds.x + border_left,
                    y: self.bounds.y + border_top,
                    width: (self.bounds.width - border_left - border_right).max(0.0),
                    height: (self.bounds.height - border_top - border_bottom).max(0.0),
                };

                match clip {
                    Some(c) => c.intersect(&padding_box),
                    None => Some(padding_box),
                }
            }
            _ => clip,
        };

        for child in self.children.iter().rev() {
            if let Some(mut path) = child.find_path_at_position(x, y, descendant_clip) {
                path.push(self.id);
                return Some(path);
            }
        }

        Some(vec![self.id])
    }
}

pub fn build_render_snapshot(root_node: Rc<RefCell<Node>>) -> RenderSnapshot {
    let (root, mut stacking_root) = build_render_tree_with_stacking(root_node);
    stacking_root.sort_by_z_index();
    let render_list = stacking_root.to_render_list();
    RenderSnapshot { root, render_list }
}

/// Builds both the `RenderNode` tree and an intermediate stacking-context tree.
///
/// The stacking-context tree contains ordered entries that can later be compiled
/// into a flat render list in draw order.
pub fn build_render_tree_with_stacking(node: Rc<RefCell<Node>>) -> (RenderNode, StackingContext) {
    let mut root_ctx = StackingContext::default();
    let tree = build_render_tree_with_ctx(node, &mut root_ctx);
    (tree, root_ctx)
}

fn build_render_tree_with_ctx(node: Rc<RefCell<Node>>, ctx: &mut StackingContext) -> RenderNode {
    let (id, bounds, style, text, text_input_state, children_nodes) = {
        let nb = node.borrow();
        (
            nb.id,
            nb.layout.bounds,
            nb.layout.style.clone(),
            nb.text.clone(),
            nb.text_input_state.clone(),
            nb.children.clone(),
        )
    };

    let mut render_children = Vec::with_capacity(children_nodes.len());

    let mut node_ctx = StackingContext {
        owner_node: Some(RenderNode {
            id,
            bounds,
            style: style.clone(),
            text: text.clone(),
            text_input_state: text_input_state.clone(),
            children: Vec::new(),
        }),
        entries: Vec::new(),
    };

    for c in children_nodes {
        let child_render = build_render_tree_with_ctx(c, &mut node_ctx);
        render_children.push(child_render);
    }

    ctx.entries.push(StackingEntry::Context(node_ctx));

    RenderNode {
        id,
        bounds,
        style,
        text,
        text_input_state,
        children: render_children,
    }
}

fn char_len(text: &str) -> usize {
    text.chars().count()
}

fn byte_index_for_char(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len())
}

#[cfg(test)]
mod asserts;

#[cfg(test)]
mod test_html;

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

#[cfg(test)]
mod flex_layout_nested_tests;

#[cfg(test)]
mod flex_layout_margin_tests;

#[cfg(test)]
mod margin_tests;

#[cfg(test)]
mod tag_selector_tests;

#[cfg(test)]
mod scroll_state_tests;

#[cfg(test)]
mod text_input_tests;
