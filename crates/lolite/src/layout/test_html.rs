use std::collections::HashMap;

use crate::{css_parser, Id};

use super::LayoutContext;

/// Loads an HTML file and looks for top level nodes with the given example_id, or \<style>.
///
/// The root element has id = 1 and the following nodes are assigned increasing Ids.
pub fn load_html_test_example(
    html: &str,
    example_id: &str,
) -> (LayoutContext, HashMap<String, Id>) {
    let mut reader = Reader::new(example_id);

    let dom = html_parser::Dom::parse(html).expect("couldn't read html test file");

    for node in &dom.children {
        reader.find_nodes(node);
    }

    reader.ctx.layout();
    (reader.ctx, reader.nodes_by_id)
}

struct Reader {
    ctx: LayoutContext,
    example_id: String,
    next_id: u64,
    nodes_by_id: HashMap<String, Id>,
}

impl Reader {
    fn new(example_id: &str) -> Self {
        Self {
            example_id: example_id.to_owned(),
            next_id: 1,
            ctx: LayoutContext::new(),
            nodes_by_id: HashMap::new(),
        }
    }

    fn find_nodes(&mut self, html_node: &html_parser::Node) {
        if let html_parser::Node::Element(element) = html_node {
            // look for id
            if let Some(id_attr) = &element.id {
                if id_attr == &self.example_id {
                    self.copy_nodes(html_node, self.ctx.document.root_id());
                }
            }

            // look for style
            if element.name == "style" {
                let text = element
                    .children
                    .get(0)
                    .and_then(|child| child.text())
                    .expect("expected style element to have a child text");

                let stylesheet = css_parser::parse_css(text).expect("expected to load stylesheet");

                self.ctx.style_sheet = stylesheet;
            }
        }
    }

    fn copy_nodes(&mut self, html_node: &html_parser::Node, parent: Id) {
        if let html_parser::Node::Element(element) = html_node {
            let id = Id::from_u64(self.next_id);
            self.next_id += 1;

            // we only support text if it's the only child
            let text = element
                .children
                .get(0)
                .and_then(|child| child.text())
                .map(|s| s.to_owned());

            let node = self.ctx.document.create_node(id, text);
            self.ctx.document.set_parent(parent, node).unwrap();

            // get id
            if let Some(id_attr) = &element.id {
                self.nodes_by_id.insert(id_attr.to_owned(), node);
            }

            // copy attributes
            if element.classes.len() > 0 {
                let class_str = element.classes.join(" ");
                self.ctx
                    .document
                    .set_attribute(node, "class".to_owned(), class_str);
            }

            for (key, value_opt) in &element.attributes {
                if let Some(value) = value_opt {
                    self.ctx
                        .document
                        .set_attribute(node, key.to_owned(), value.to_owned());
                }
            }

            // store in map if it has an id
            if let Some(id_attr) = &element.id {
                self.nodes_by_id.insert(id_attr.to_owned(), node);
            }

            // check multiple text children
            if element
                .children
                .iter()
                .filter(|child| child.text().is_some())
                .count()
                > 1
            {
                panic!("multiple text children not supported");
            }

            // recurse into children
            for child in &element.children {
                self.copy_nodes(child, node);
            }
        }
    }
}
