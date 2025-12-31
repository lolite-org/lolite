use lolite::Engine;

fn main() {
    // Create a thread-safe CSS engine
    let engine = Engine::new();

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

    // Run
    let params = lolite::Params {
        on_click: Some(Box::new(|x, y, elements| {
            println!("Clicked at ({}, {}), elements: {:?}", x, y, elements);
        })),
    };

    if let Err(e) = engine.run(params) {
        eprintln!("Error encountered: {:?}", e);
    }
}
