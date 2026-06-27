use crate::engine_backend::{EngineBackend, SonateEventCallback, SonateId};
use crate::{EngineHandle, SonateEvent, SonateEventType};
use sonate::{Engine, Id, Params};
use std::ffi::c_void;
use std::ptr;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct EventCallbackState {
    callback: SonateEventCallback,
    user_data: *mut c_void,
}

unsafe impl Send for EventCallbackState {}

pub struct DirectBackend {
    handle: EngineHandle,
    engine: Engine,
    callback_state: Arc<Mutex<EventCallbackState>>,
}

impl DirectBackend {
    pub fn new(handle: EngineHandle) -> Self {
        Self {
            handle,
            engine: Engine::new(),
            callback_state: Arc::new(Mutex::new(EventCallbackState::default())),
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

    fn destroy_node(&self, node_id: SonateId) {
        self.engine.remove_node(Id::from_u64(node_id));
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

    fn set_event_callback(&self, callback: SonateEventCallback, user_data: *mut c_void) -> i32 {
        let mut state = self.callback_state.lock().unwrap();
        state.callback = callback;
        state.user_data = user_data;
        0
    }

    fn run(&self) -> i32 {
        let click_callback_state = Arc::clone(&self.callback_state);
        let scroll_callback_state = Arc::clone(&self.callback_state);
        let handle = self.handle;

        match self.engine.run(Params {
            on_click: Some(Box::new(move |x, y, elements| {
                let (callback, user_data) = {
                    let state = click_callback_state.lock().unwrap();
                    (state.callback, state.user_data)
                };

                let Some(callback) = callback else {
                    return;
                };

                let element_ids: Vec<SonateId> = elements.iter().map(|id| id.as_u64()).collect();
                let event = SonateEvent {
                    event_type: SonateEventType::Click,
                    x,
                    y,
                    scroll_target_id: 0,
                    scroll_dx: 0.0,
                    scroll_dy: 0.0,
                    element_ids: if element_ids.is_empty() {
                        ptr::null()
                    } else {
                        element_ids.as_ptr()
                    },
                    element_count: element_ids.len(),
                };

                callback(handle, &event, user_data);
            })),
            on_scroll: Some(Box::new(move |target_id, dx, dy| {
                let (callback, user_data) = {
                    let state = scroll_callback_state.lock().unwrap();
                    (state.callback, state.user_data)
                };

                let Some(callback) = callback else {
                    return;
                };

                let event = SonateEvent {
                    event_type: SonateEventType::Scroll,
                    x: 0.0,
                    y: 0.0,
                    scroll_target_id: target_id.as_u64(),
                    scroll_dx: dx,
                    scroll_dy: dy,
                    element_ids: ptr::null(),
                    element_count: 0,
                };

                callback(handle, &event, user_data);
            })),
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
