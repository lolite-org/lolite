use std::cell::RefCell;

mod engine;
mod flex_layout;
mod painter;
mod plumbing;

fn main() -> anyhow::Result<()> {
    let mut engine = engine::Engine::new();

    // style
    engine.style_sheet.add_rule(engine::Rule {
        selector: engine::Selector::Class("flex_container".to_owned()),
        declarations: vec![engine::Style {
            display: engine::Display::Flex,
            flex_direction: Some(engine::FlexDirection::Row),
            gap: Some(engine::Length::Px(10.0)),
            padding: Some(engine::Extend {
                top: engine::Length::Px(10.0),
                right: engine::Length::Px(10.0),
                bottom: engine::Length::Px(10.0),
                left: engine::Length::Px(10.0),
            }),
            ..Default::default()
        }],
    });

    engine.style_sheet.add_rule(engine::Rule {
        selector: engine::Selector::Class("red_box".to_owned()),
        declarations: vec![engine::Style {
            background_color: Some(engine::Rgba {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            }),
            border_radius: Some(engine::BorderRadius {
                top_left: engine::Length::Px(4.0),
                top_right: engine::Length::Px(4.0),
                bottom_right: engine::Length::Px(4.0),
                bottom_left: engine::Length::Px(4.0),
            }),
            border_width: Some(engine::Length::Px(2.0)),
            border_color: Some(engine::Rgba {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            }),
            margin: Some(engine::Extend {
                top: engine::Length::Px(10.0),
                right: engine::Length::Px(10.0),
                bottom: engine::Length::Px(10.0),
                left: engine::Length::Px(10.0),
            }),
            ..Default::default()
        }],
    });

    engine.style_sheet.add_rule(engine::Rule {
        selector: engine::Selector::Class("green_box".to_owned()),
        declarations: vec![engine::Style {
            background_color: Some(engine::Rgba {
                r: 0,
                g: 255,
                b: 0,
                a: 255,
            }),
            border_radius: Some(engine::BorderRadius {
                top_left: engine::Length::Px(8.0),
                top_right: engine::Length::Px(8.0),
                bottom_right: engine::Length::Px(8.0),
                bottom_left: engine::Length::Px(8.0),
            }),
            ..Default::default()
        }],
    });

    // document
    let document = &mut engine.document;

    let root = document.root_id();
    let a = document.create_node_autoid(Some("Hello".to_string()));
    let b = document.create_node_autoid(Some("World".to_string()));
    let c = document.create_node_autoid(Some("xD".to_string()));
    document.set_parent(root, a).unwrap();
    document.set_parent(root, b).unwrap();
    document.set_parent(root, c).unwrap();
    document.set_attribute(root, "class".to_owned(), "flex_container".to_owned());
    document.set_attribute(a, "class".to_owned(), "red_box".to_owned());
    document.set_attribute(b, "class".to_owned(), "green_box".to_owned());

    // run
    let params = plumbing::Params {
        on_draw: Box::new(move |canvas| {
            engine.layout();

            // Debug: Print layout bounds to verify margin implementation
            let root_node = engine.document.root_node();
            println!(
                "Root bounds: x={}, y={}, w={}, h={}",
                root_node.borrow().layout.bounds.x,
                root_node.borrow().layout.bounds.y,
                root_node.borrow().layout.bounds.width,
                root_node.borrow().layout.bounds.height
            );

            for (i, child) in root_node.borrow().children.iter().enumerate() {
                let bounds = &child.borrow().layout.bounds;
                let class = child
                    .borrow()
                    .attributes
                    .get("class")
                    .unwrap_or(&"none".to_string())
                    .clone();
                println!(
                    "Child {} ({}): x={}, y={}, w={}, h={}",
                    i, class, bounds.x, bounds.y, bounds.width, bounds.height
                );
            }

            let mut painter = painter::Painter::new(canvas);
            painter.paint(&engine.document);
        }),
    };

    plumbing::run_gui(&RefCell::new(params))?;

    Ok(())
}
