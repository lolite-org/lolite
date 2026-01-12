use crate::{
    flex_layout::FlexLayoutEngine,
    style::{BoxSizing, Length, Selector, Style, StyleSheet},
    text::{default_text_measurer, FontSpec, TextMeasurer},
    Id,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc};

#[derive(Default)]
pub struct Layout {
    pub bounds: Rect,
    pub style: Arc<Style>,
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
}

pub struct LayoutContext {
    pub document: Document,
    pub style_sheet: StyleSheet,
    flex_layout_engine: FlexLayoutEngine,
    pub text_measurer: Arc<dyn TextMeasurer>,
}

impl LayoutContext {
    pub fn new() -> Self {
        Self {
            document: Document::new(),
            style_sheet: StyleSheet::new(),
            flex_layout_engine: FlexLayoutEngine::new(),
            text_measurer: default_text_measurer(),
        }
    }

    pub fn layout(&mut self) {
        self.text_measurer.begin_layout_pass();
        self.layout_node(self.document.root.clone(), 0.0, 0.0);
        self.text_measurer.end_layout_pass_and_sweep();
    }

    pub fn layout_node(&self, node: Rc<RefCell<Node>>, x: f64, y: f64) {
        // Get style for this node - merge existing style with CSS rules
        let style = {
            let node_borrow = node.borrow();
            // Start with existing style as base (this preserves manually set properties like flex_wrap)
            let mut style = node_borrow.layout.style.as_ref().clone();

            // Apply CSS rules on top of existing style.
            // The `class` attribute is treated as a whitespace-separated list of classes.
            if let Some(class_attr) = node_borrow.attributes.get("class") {
                for class_name in class_attr.split_whitespace() {
                    let selector = Selector::Class(class_name.to_string());

                    if let Some(rule) = self
                        .style_sheet
                        .rules
                        .iter()
                        .find(|rule| rule.selector == selector)
                    {
                        for declaration in &rule.declarations {
                            style.merge(declaration);
                        }
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
        let is_text_node = node.borrow().is_text_node();

        // Lolite stores `layout.bounds` as the element's border-box.
        // `box-sizing` determines whether CSS `width/height` refer to the content-box or border-box.
        let resolved_box_sizing = style.box_sizing.unwrap_or(BoxSizing::ContentBox);
        let padding = style.padding.clone().unwrap_or_default();
        let padding_w = padding.left.to_px() + padding.right.to_px();
        let padding_h = padding.top.to_px() + padding.bottom.to_px();
        let border_sum = style.border_width.map(|w| w.to_px() * 2.0).unwrap_or(0.0);

        let resolve_border_box =
            |specified: Option<Length>, fallback: f64, padding_sum: f64| -> f64 {
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
                        fallback_width_border_box = text_size.width + padding_w + border_sum;
                    }

                    // Height: if not specified, try to wrap to a specified width (if any), else unwrapped.
                    if matches!(style.height, Some(Length::Auto)) {
                        let text_size = match style.width {
                            Some(Length::Px(specified_width_px)) if specified_width_px > 0.0 => {
                                // Wrap within the content box width.
                                let content_max_width = match resolved_box_sizing {
                                    BoxSizing::ContentBox => specified_width_px,
                                    BoxSizing::BorderBox => {
                                        (specified_width_px - padding_w - border_sum).max(0.0)
                                    }
                                };
                                self.text_measurer
                                    .measure_wrapped(text, &font, content_max_width)
                            }
                            _ => self.text_measurer.measure_unwrapped(text, &font),
                        };

                        fallback_height_border_box = text_size.height + padding_h + border_sum;
                    }
                }
            }

            let mut node_borrow = node.borrow_mut();
            node_borrow.layout.bounds.width =
                resolve_border_box(style.width, fallback_width_border_box, padding_w);
            node_borrow.layout.bounds.height =
                resolve_border_box(style.height, fallback_height_border_box, padding_h);
            node_borrow.layout.style = Arc::new(style);
        } else {
            // Container node - handle flexbox layout
            let container_width = resolve_border_box(style.width, 800.0, padding_w);
            let container_height = resolve_border_box(style.height, 500.0, padding_h);

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

impl RenderNode {
    /// Find the element at the given position (x, y).
    ///
    /// Returns a `Vec<Id>` where the first element is the topmost element at the position,
    /// and subsequent elements are its parents up to the root.
    /// This enables event bubbling by providing the full parent chain.
    pub fn find_element_at_position(&self, x: f64, y: f64) -> Vec<Id> {
        self.find_path_at_position(x, y).unwrap_or_default()
    }

    fn find_path_at_position(&self, x: f64, y: f64) -> Option<Vec<Id>> {
        if !self.bounds.contains_point(x, y) {
            return None;
        }

        for child in self.children.iter().rev() {
            if let Some(mut path) = child.find_path_at_position(x, y) {
                path.push(self.id);
                return Some(path);
            }
        }

        Some(vec![self.id])
    }
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
