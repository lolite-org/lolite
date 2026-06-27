use std::collections::HashMap;
use std::ffi::c_void;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

mod direct_backend;
mod engine_backend;
mod worker_backend;

use direct_backend::DirectBackend;
use engine_backend::EngineBackend;
use worker_backend::WorkerBackend;

/// Handle type for engine instances
pub type EngineHandle = usize;

/// ID type for nodes and other engine-owned objects.
pub type SonateId = u64;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub enum SonateEventType {
    Click = 1,
    Scroll = 2,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SonateEvent {
    pub event_type: SonateEventType,
    pub x: f64,
    pub y: f64,
    pub scroll_target_id: SonateId,
    pub scroll_dx: f64,
    pub scroll_dy: f64,
    pub element_ids: *const SonateId,
    pub element_count: usize,
}

type EngineBox = Box<dyn EngineBackend>;
type EngineRef = Arc<Mutex<EngineBox>>;

static ENGINE_INSTANCES: std::sync::LazyLock<Mutex<HashMap<EngineHandle, EngineRef>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

static NEXT_HANDLE: AtomicUsize = AtomicUsize::new(1);

/// Initialize the sonate engine
///
/// # Arguments
/// * `use_same_process` - If true, runs in same process (more performant).
///                       If false, creates a worker process (for cases where UI must run on main thread)
///
/// # Returns
/// * Engine handle on success, 0 on error
#[no_mangle]
pub extern "C" fn sonate_init(use_same_process: bool) -> EngineHandle {
    let handle = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);

    let backend: EngineBox = if use_same_process {
        Box::new(DirectBackend::new(handle))
    } else {
        match WorkerBackend::new(handle) {
            Ok(b) => Box::new(b),
            Err(e) => {
                eprintln!("Failed to create worker instance: {}", e);
                return 0;
            }
        }
    };

    ENGINE_INSTANCES
        .lock()
        .unwrap()
        .insert(handle, Arc::new(Mutex::new(backend)));

    handle
}

#[no_mangle]
pub extern "C" fn sonate_init_internal(handle: EngineHandle) {
    ENGINE_INSTANCES.lock().unwrap().insert(
        handle,
        Arc::new(Mutex::new(Box::new(DirectBackend::new(handle)))),
    );
}

fn get_engine(handle: EngineHandle) -> Option<EngineRef> {
    ENGINE_INSTANCES.lock().unwrap().get(&handle).cloned()
}

/// Add a CSS stylesheet to the engine
///
/// # Arguments
/// * `handle` - Engine handle returned from sonate_init
/// * `css_content` - Null-terminated CSS string
#[no_mangle]
pub extern "C" fn sonate_add_stylesheet(handle: EngineHandle, css_content: *const c_char) {
    if handle == 0 {
        eprintln!("Invalid engine handle");
        return;
    }

    if css_content.is_null() {
        eprintln!("CSS content is null");
        return;
    }

    let css_str = match unsafe { CStr::from_ptr(css_content) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            eprintln!("Invalid UTF-8 in CSS content: {}", e);
            return;
        }
    };

    let Some(engine) = get_engine(handle) else {
        eprintln!("Engine handle not found");
        return;
    };

    engine.lock().unwrap().add_stylesheet(css_str);
}

/// Create a new document node
///
/// # Arguments
/// * `handle` - Engine handle returned from sonate_init
/// * `text_content` - Optional null-terminated text content (can be null)
///
/// # Returns
/// * Node ID on success, 0 on error (since root is always ID 0, we can distinguish)
#[no_mangle]
pub extern "C" fn sonate_create_node(
    handle: EngineHandle,
    node_id: SonateId,
    text_content: *const c_char,
) -> SonateId {
    if handle == 0 {
        eprintln!("Invalid engine handle");
        return 0;
    }

    if node_id == 0 {
        eprintln!("Invalid node id (0 is reserved for root)");
        return 0;
    }

    let text = if text_content.is_null() {
        None
    } else {
        match unsafe { CStr::from_ptr(text_content) }.to_str() {
            Ok(s) => Some(s.to_string()),
            Err(e) => {
                eprintln!("Invalid UTF-8 in text content: {}", e);
                return 0;
            }
        }
    };

    let Some(engine) = get_engine(handle) else {
        eprintln!("Engine handle not found");
        return 0;
    };

    engine.lock().unwrap().create_node(node_id, text);
    node_id
}

/// Set parent-child relationship between nodes
///
/// # Arguments
/// * `handle` - Engine handle returned from sonate_init
/// * `parent_id` - ID of the parent node
/// * `child_id` - ID of the child node
///
/// # Returns
/// * 0 on success, -1 on error
#[no_mangle]
pub extern "C" fn sonate_set_parent(handle: EngineHandle, parent_id: SonateId, child_id: SonateId) {
    if handle == 0 {
        eprintln!("Invalid engine handle");
        return;
    }

    let Some(engine) = get_engine(handle) else {
        eprintln!("Engine handle not found");
        return;
    };

    engine.lock().unwrap().set_parent(parent_id, child_id);
}

/// Set an attribute on a node
///
/// # Arguments
/// * `handle` - Engine handle returned from sonate_init
/// * `node_id` - ID of the node
/// * `key` - Null-terminated attribute key string
/// * `value` - Null-terminated attribute value string
///
/// # Returns
/// * 0 on success, -1 on error
#[no_mangle]
pub extern "C" fn sonate_set_attribute(
    handle: EngineHandle,
    node_id: SonateId,
    key: *const c_char,
    value: *const c_char,
) {
    if handle == 0 {
        eprintln!("Invalid engine handle");
        return;
    }

    if key.is_null() || value.is_null() {
        eprintln!("Key or value is null");
        return;
    }

    let key_str = match unsafe { CStr::from_ptr(key) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            eprintln!("Invalid UTF-8 in attribute key: {}", e);
            return;
        }
    };

    let value_str = match unsafe { CStr::from_ptr(value) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            eprintln!("Invalid UTF-8 in attribute value: {}", e);
            return;
        }
    };

    let Some(engine) = get_engine(handle) else {
        eprintln!("Engine handle not found");
        return;
    };

    engine
        .lock()
        .unwrap()
        .set_attribute(node_id, key_str, value_str);
}

/// Remove a node and its descendants from the document.
///
/// # Arguments
/// * `handle` - Engine handle returned from sonate_init
/// * `node_id` - ID of the node to remove
#[no_mangle]
pub extern "C" fn sonate_destroy_node(handle: EngineHandle, node_id: SonateId) {
    if handle == 0 {
        eprintln!("Invalid engine handle");
        return;
    }

    if node_id == 0 {
        eprintln!("Invalid node id (0 is reserved for root)");
        return;
    }

    let Some(engine) = get_engine(handle) else {
        eprintln!("Engine handle not found");
        return;
    };

    engine.lock().unwrap().destroy_node(node_id);
}

/// Get the root node ID of the document
///
/// # Arguments
/// * `handle` - Engine handle returned from sonate_init
///
/// # Returns
/// * Root node ID (always 0 for the document root), or 0 if handle is invalid
#[no_mangle]
pub extern "C" fn sonate_root_id(handle: EngineHandle) -> SonateId {
    if handle == 0 {
        eprintln!("Invalid engine handle");
        return 0;
    }

    let Some(engine) = get_engine(handle) else {
        eprintln!("Engine handle not found");
        return 0;
    };

    let id = engine.lock().unwrap().root_id();
    id
}

/// Register a generic event callback.
///
/// Returns:
/// * 0 on success, -1 on error
#[no_mangle]
pub extern "C" fn sonate_set_event_callback(
    handle: EngineHandle,
    callback: engine_backend::SonateEventCallback,
    user_data: *mut c_void,
) -> c_int {
    if handle == 0 {
        eprintln!("Invalid engine handle");
        return -1;
    }

    let Some(engine) = get_engine(handle) else {
        eprintln!("Engine handle not found");
        return -1;
    };

    let result = engine
        .lock()
        .unwrap()
        .set_event_callback(callback, user_data);

    result
}

/// Run the engine event loop (blocking).
///
/// # Arguments
/// * `handle` - Engine handle returned from sonate_init
///
/// # Returns
/// * 0 on success, -1 on error
#[no_mangle]
pub extern "C" fn sonate_run(handle: EngineHandle) -> c_int {
    if handle == 0 {
        eprintln!("Invalid engine handle");
        return -1;
    }

    let Some(engine) = get_engine(handle) else {
        eprintln!("Engine handle not found");
        return -1;
    };

    let code = engine.lock().unwrap().run();
    code
}

/// Cleanup and destroy an engine instance
///
/// # Arguments
/// * `handle` - Engine handle returned from sonate_init
///
/// # Returns
/// * 0 on success, -1 on error
#[no_mangle]
pub extern "C" fn sonate_destroy(handle: EngineHandle) -> c_int {
    if handle == 0 {
        eprintln!("Invalid engine handle");
        return -1;
    }

    let engine = ENGINE_INSTANCES.lock().unwrap().remove(&handle);
    let Some(engine) = engine else {
        eprintln!("Engine handle not found");
        return -1;
    };

    let code = engine.lock().unwrap().destroy();
    code
}
