use lolite_css::{CssEngine, Params};
use std::cell::RefCell;

fn main() -> anyhow::Result<()> {
    // Create a thread-safe CSS engine
    let engine = CssEngine::new();

    // Example: Parse CSS from a string
    let css_content = r#"
        .flex_container {
            display: flex;
            flex-direction: row;
            gap: 10px;
            padding: 10px;
        }

        .red_box {
            background-color: #ff0000;
            border-width: 2px;
            border-color: black;
            border-radius: 8px;
            margin: 10px;
        }

        .green_box {
            background-color: green;
            border-radius: 12px 4px;
        }
    "#;

    // Parse the CSS and load it into the engine
    engine.add_stylesheet(css_content);

    // Create document structure
    let root = engine.root_id();
    let a = engine.create_node(Some("Hello".to_string()));
    let b = engine.create_node(Some("World".to_string()));
    let c = engine.create_node(Some("xD".to_string()));

    engine.set_parent(root, a);
    engine.set_parent(root, b);
    engine.set_parent(root, c);

    engine.set_attribute(root, "class".to_owned(), "flex_container".to_owned());
    engine.set_attribute(a, "class".to_owned(), "red_box".to_owned());
    engine.set_attribute(b, "class".to_owned(), "green_box".to_owned());

    // Setup windowing callbacks
    let engine_for_draw = engine.clone();
    let engine_for_click = engine.clone();

    let params = Params {
        on_draw: Box::new(move |canvas| {
            if let Some(snapshot) = engine_for_draw.get_current_snapshot() {
                let mut painter = lolite_css::painter::Painter::new(canvas);
                painter.paint(&snapshot);
            }
        }),
        on_click: Some(Box::new(move |x, y| {
            // Perform hit testing
            let elements = engine_for_click.find_element_at_position(x, y);

            if elements.is_empty() {
                println!("Click detected on background at ({:.1}, {:.1})", x, y);
            } else {
                println!(
                    "Click detected at ({:.1}, {:.1}) on {} elements:",
                    x,
                    y,
                    elements.len()
                );
                for (i, element_id) in elements.iter().enumerate() {
                    println!("  Level {}: Element ID {:?}", i, element_id.value());
                }
            }
        })),
    };

    lolite_css::run(&RefCell::new(params))?;

    Ok(())
}
