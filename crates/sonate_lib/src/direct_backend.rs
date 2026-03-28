use crate::engine_backend::{EngineBackend, SonateId};
use sonate::{Engine, Id, Params};

pub struct DirectBackend {
    engine: Engine,
}

impl DirectBackend {
    pub fn new() -> Self {
        Self {
            engine: Engine::new(),
        }
    }
}

impl EngineBackend for DirectBackend {
    fn add_stylesheet(&self, css: String) {
        self.engine.add_stylesheet(&css);
    }

    fn create_node(&self, node_id: SonateId, text: Option<String>) {
        let _ = self.engine.create_node(Id::from_u64(node_id), text);
    }

    fn set_parent(&self, parent_id: SonateId, child_id: SonateId) {
        self.engine
            .set_parent(Id::from_u64(parent_id), Id::from_u64(child_id));
    }

    fn set_attribute(&self, node_id: SonateId, key: String, value: String) {
        self.engine.set_attribute(Id::from_u64(node_id), key, value);
    }

    fn root_id(&self) -> SonateId {
        self.engine.root_id().as_u64()
    }

    fn run(&self) -> i32 {
        match self.engine.run(Params {
            on_click: None,
            on_scroll: None,
        }) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("sonate_run failed: {:?}", err);
                -1
            }
        }
    }

    fn destroy(&self) -> i32 {
        0
    }
}
