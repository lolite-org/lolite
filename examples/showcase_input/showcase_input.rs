use sonate::{Engine, Id, Params};

fn next_id() -> Id {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    Id::from_u64(COUNTER.fetch_add(1, Ordering::SeqCst))
}

fn main() {
    let engine = Engine::new();

    engine.add_stylesheet(
        r#"
        input {
            -sonate-widget: text-input;
            width: 320px;
            height: 40px;
            padding: 10px;
            margin: 20px;
            border: 1px solid #444444;
            background-color: #ffffff;
            color: #111111;
        }
        "#,
    );

    let root = engine.root_id();
    let input = engine.create_node(next_id(), None);
    engine.set_parent(root, input);
    engine.set_attribute(input, "tag".to_owned(), "input".to_owned());
    engine.set_attribute(input, "value".to_owned(), "Type here".to_owned());

    let params = Params {
        on_click: None,
        on_scroll: None,
    };

    if let Err(error) = engine.run(params) {
        eprintln!("Error encountered: {:?}", error);
    }
}
