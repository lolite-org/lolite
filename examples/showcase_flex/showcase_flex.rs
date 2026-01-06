use std::{cell::RefCell, rc::Rc};

use lolite::{Engine, Id, Params};

fn next_id() -> Id {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    Id::from_u64(COUNTER.fetch_add(1, Ordering::SeqCst))
}

#[derive(Default)]
enum JustifyContent {
    #[default]
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Default)]
enum AlignItems {
    #[default]
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Stretch,
}

#[derive(Default)]
enum AlignContent {
    #[default]
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
    SpaceBetween,
    SpaceAround,
}

#[derive(Default)]
enum Gap {
    #[default]
    None,
    Px20,
    Px10px20,
}

#[derive(Default)]
struct State {
    wrap: bool,
    direction: bool,
    justify_content: JustifyContent,
    align_items: AlignItems,
    align_self: AlignItems,
    align_content: AlignContent,
    gap: Gap,
    order: bool,
    grow: bool,
    shrink: bool,
    basis: bool,
    flex: bool,
}

fn apply_state(
    engine: &Engine,
    state: &State,
    flex_container: Id,
    item1: Id,
    item2: Id,
    item3: Id,
) {
    let justify_class = match state.justify_content {
        JustifyContent::FlexStart => "justify-flex-start",
        JustifyContent::FlexEnd => "justify-flex-end",
        JustifyContent::Center => "justify-center",
        JustifyContent::SpaceBetween => "justify-space-between",
        JustifyContent::SpaceAround => "justify-space-around",
        JustifyContent::SpaceEvenly => "justify-space-evenly",
    };

    let align_items_class = match state.align_items {
        AlignItems::Stretch => "items-stretch",
        AlignItems::FlexStart => "items-flex-start",
        AlignItems::FlexEnd => "items-flex-end",
        AlignItems::Center => "items-center",
        AlignItems::Baseline => "items-baseline",
    };

    let align_content_class = match state.align_content {
        AlignContent::Stretch => "content-stretch",
        AlignContent::FlexStart => "content-flex-start",
        AlignContent::FlexEnd => "content-flex-end",
        AlignContent::Center => "content-center",
        AlignContent::SpaceBetween => "content-space-between",
        AlignContent::SpaceAround => "content-space-around",
    };

    let gap_class = match state.gap {
        Gap::None => "gap-none",
        Gap::Px20 => "gap-20",
        Gap::Px10px20 => "gap-row10-col20",
    };

    let mut container_classes = Vec::new();
    container_classes.push("flex_container");
    container_classes.push(if state.wrap { "wrap" } else { "no-wrap" });
    container_classes.push(if state.direction { "column" } else { "row" });
    container_classes.push(justify_class);
    container_classes.push(align_items_class);
    container_classes.push(align_content_class);
    container_classes.push(gap_class);

    engine.set_attribute(
        flex_container,
        "class".to_owned(),
        container_classes.join(" "),
    );

    println!("Applied classes to container: {:?}", container_classes);

    // Demonstrate item-level properties on the first item.
    let self_class = match state.align_self {
        AlignItems::Stretch => "self-stretch",
        AlignItems::FlexStart => "self-flex-start",
        AlignItems::FlexEnd => "self-flex-end",
        AlignItems::Center => "self-center",
        AlignItems::Baseline => "self-baseline",
    };

    let mut item1_classes = Vec::new();
    item1_classes.push("box");
    item1_classes.push("red_box");
    item1_classes.push(self_class);
    if state.grow {
        item1_classes.push("item-grow");
    }
    if state.shrink {
        item1_classes.push("item-shrink-2");
    }
    if state.basis {
        item1_classes.push("item-basis-220");
    }
    if state.flex {
        item1_classes.push("item-flex-1");
    }

    // Implement order via the CSS `order` property.
    if state.order {
        item1_classes.push("order-3");
    } else {
        item1_classes.push("order-1");
    }

    engine.set_attribute(item1, "class".to_owned(), item1_classes.join(" "));

    let item2_order = if state.order { "order-2" } else { "order-2" };
    let item3_order = if state.order { "order-1" } else { "order-3" };
    engine.set_attribute(
        item2,
        "class".to_owned(),
        format!("box green_box {}", item2_order),
    );
    engine.set_attribute(
        item3,
        "class".to_owned(),
        format!("box blue_box {}", item3_order),
    );
}

fn div(engine: &Engine, text: Option<String>, parent: Id, class: &str) -> Id {
    let node = engine.create_node(next_id(), text);
    engine.set_parent(parent, node);
    engine.set_attribute(node, "class".to_owned(), class.to_owned());
    node
}

fn main() {
    // Create a thread-safe CSS engine
    let engine = Engine::new();

    // Example: Parse CSS from a string
    let css_content = r#"
.buttons {
    display: flex;
    flex-direction: row;
    flex-wrap: wrap;
    gap: 10px;
    padding: 10px;
    background-color: #bbbbbb;
}

.button {
    padding: 10px 20px;
    border-radius: 5px;
    border-width: 1px;
    border-color: #0056b3;
    background-color: #dde9ff;
    width: 200px;
}

.flex_container {
    display: flex;
    flex-direction: row;
    padding: 10px;
    width: 700px;
    height: 260px;
    background-color: #eeeeee;
}

.wrap { flex-wrap: wrap; }
.no-wrap { flex-wrap: nowrap; }

.row { flex-direction: row; }
.column { flex-direction: column; }

.justify-flex-start { justify-content: flex-start; }
.justify-flex-end { justify-content: flex-end; }
.justify-center { justify-content: center; }
.justify-space-between { justify-content: space-between; }
.justify-space-around { justify-content: space-around; }
.justify-space-evenly { justify-content: space-evenly; }

.items-stretch { align-items: stretch; }
.items-flex-start { align-items: flex-start; }
.items-flex-end { align-items: flex-end; }
.items-center { align-items: center; }
.items-baseline { align-items: baseline; }

.content-stretch { align-content: stretch; }
.content-flex-start { align-content: flex-start; }
.content-flex-end { align-content: flex-end; }
.content-center { align-content: center; }
.content-space-between { align-content: space-between; }
.content-space-around { align-content: space-around; }

.self-stretch { align-self: stretch; }
.self-flex-start { align-self: flex-start; }
.self-flex-end { align-self: flex-end; }
.self-center { align-self: center; }
.self-baseline { align-self: baseline; }

.item-grow { flex-grow: 1; }
.item-shrink-2 { flex-shrink: 2; }
.item-basis-220 { flex-basis: 220px; }
.item-flex-1 { flex-grow: 1; flex-shrink: 1; flex-basis: 0px; }

.order-1 { order: 1; }
.order-2 { order: 2; }
.order-3 { order: 3; }

.gap-none { gap: 0px; }
.gap-20 { gap: 20px; }
.gap-row10-col20 { row-gap: 10px; column-gap: 20px; }

.box {
    width: 120px;
    height: 60px;
    margin: 10px;
    border-width: 2px;
    border-color: #000000;
    border-radius: 8px;
}

.red_box {
    background-color: #ff0000;
}

.green_box {
    background-color: green;
    border-radius: 12px 4px;
}

.blue_box {
    background-color: #0000ff;
    border-radius: 4px 12px;
}
    "#;

    // Parse the CSS and load it into the engine
    engine.add_stylesheet(css_content);

    // Create document structure
    let root = engine.root_id();

    // State
    let state = Rc::new(RefCell::new(State::default()));

    // Buttons
    let top_box = div(&engine, None, root, "buttons");
    let wrap_button = div(&engine, Some("Wrap".to_string()), top_box, "button");
    let direction_button = div(&engine, Some("Direction".to_string()), top_box, "button");
    let justify_content_button = div(
        &engine,
        Some("Justify Content".to_string()),
        top_box,
        "button",
    );

    let align_items_button = div(&engine, Some("Align Items".to_string()), top_box, "button");
    let align_content_button = div(
        &engine,
        Some("Align Content".to_string()),
        top_box,
        "button",
    );
    let align_self_button = div(&engine, Some("Align Self".to_string()), top_box, "button");
    let gap_button = div(&engine, Some("Gap".to_string()), top_box, "button");
    let order_button = div(&engine, Some("Order".to_string()), top_box, "button");
    let grow_button = div(&engine, Some("Grow".to_string()), top_box, "button");
    let shrink_button = div(&engine, Some("Shrink".to_string()), top_box, "button");
    let basis_button = div(&engine, Some("Basis".to_string()), top_box, "button");
    let flex_button = div(&engine, Some("Flex".to_string()), top_box, "button");

    // Example
    let flex_container = div(&engine, None, root, "flex_container");

    let item1 = div(
        &engine,
        Some("First Item".to_string()),
        flex_container,
        "box red_box",
    );
    let item2 = div(
        &engine,
        Some("Second Item".to_string()),
        flex_container,
        "box green_box",
    );
    let item3 = div(
        &engine,
        Some("Third Item".to_string()),
        flex_container,
        "box blue_box",
    );

    // Initial state application
    apply_state(
        &engine,
        &state.borrow(),
        flex_container,
        item1,
        item2,
        item3,
    );

    // Run
    let params = Params {
        on_click: {
            let engine = engine.clone();
            let state = state.clone();

            Some(Box::new(move |_x, _y, elements| {
                let mut state = state.borrow_mut();

                if elements.first() == Some(&wrap_button) {
                    state.wrap = !state.wrap;
                } else if elements.first() == Some(&direction_button) {
                    state.direction = !state.direction;
                } else if elements.first() == Some(&justify_content_button) {
                    state.justify_content = match state.justify_content {
                        JustifyContent::FlexStart => JustifyContent::FlexEnd,
                        JustifyContent::FlexEnd => JustifyContent::Center,
                        JustifyContent::Center => JustifyContent::SpaceBetween,
                        JustifyContent::SpaceBetween => JustifyContent::SpaceAround,
                        JustifyContent::SpaceAround => JustifyContent::SpaceEvenly,
                        JustifyContent::SpaceEvenly => JustifyContent::FlexStart,
                    };
                } else if elements.first() == Some(&align_items_button) {
                    state.align_items = match state.align_items {
                        AlignItems::Stretch => AlignItems::FlexStart,
                        AlignItems::FlexStart => AlignItems::FlexEnd,
                        AlignItems::FlexEnd => AlignItems::Center,
                        AlignItems::Center => AlignItems::Baseline,
                        AlignItems::Baseline => AlignItems::Stretch,
                    };
                } else if elements.first() == Some(&align_content_button) {
                    state.align_content = match state.align_content {
                        AlignContent::Stretch => AlignContent::FlexStart,
                        AlignContent::FlexStart => AlignContent::FlexEnd,
                        AlignContent::FlexEnd => AlignContent::Center,
                        AlignContent::Center => AlignContent::SpaceBetween,
                        AlignContent::SpaceBetween => AlignContent::SpaceAround,
                        AlignContent::SpaceAround => AlignContent::Stretch,
                    };
                } else if elements.first() == Some(&align_self_button) {
                    state.align_self = match state.align_self {
                        AlignItems::Stretch => AlignItems::FlexStart,
                        AlignItems::FlexStart => AlignItems::FlexEnd,
                        AlignItems::FlexEnd => AlignItems::Center,
                        AlignItems::Center => AlignItems::Baseline,
                        AlignItems::Baseline => AlignItems::Stretch,
                    };
                } else if elements.first() == Some(&gap_button) {
                    state.gap = match state.gap {
                        Gap::None => Gap::Px20,
                        Gap::Px20 => Gap::Px10px20,
                        Gap::Px10px20 => Gap::None,
                    };
                } else if elements.first() == Some(&order_button) {
                    state.order = !state.order;
                } else if elements.first() == Some(&grow_button) {
                    state.grow = !state.grow;
                } else if elements.first() == Some(&shrink_button) {
                    state.shrink = !state.shrink;
                } else if elements.first() == Some(&basis_button) {
                    state.basis = !state.basis;
                } else if elements.first() == Some(&flex_button) {
                    state.flex = !state.flex;
                }

                apply_state(&engine, &state, flex_container, item1, item2, item3);
            }))
        },
    };

    if let Err(e) = engine.run(params) {
        eprintln!("Error encountered: {:?}", e);
    }
}
