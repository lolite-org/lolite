use lolite::{CssEngine, Id};
use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

mod worker_instance;

/// Engine wrapper that can operate in two modes:
/// - Same process: Direct CssEngine usage
/// - Worker process: Communication through WorkerInstance
enum EngineMode {
    SameProcess(Box<CssEngine>),
    WorkerProcess(worker_instance::WorkerInstance),
}

/// Handle type for engine instances
pub type EngineHandle = usize;

/// Thread-safe global storage for engine instances
/// We use Box<CssEngine> to store on heap and avoid Send+Sync requirements for static storage
/// Note: While CssEngine has internal synchronization via FairMutex, its inner types (Rc<RefCell<>>)
/// are not Send+Sync, so we need to be careful about cross-thread access.
/// Each engine should be accessed from the thread that created it, or proper synchronization
/// should be ensured by the caller.
static ENGINE_INSTANCES: std::sync::LazyLock<Mutex<HashMap<EngineHandle, EngineMode>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

static NEXT_HANDLE: AtomicUsize = AtomicUsize::new(1);

/// Initialize the lolite engine
///
/// # Arguments
/// * `use_same_process` - If true, runs in same process (more performant).
///                       If false, creates a worker process (for cases where UI must run on main thread)
///
/// # Returns
/// * Engine handle on success, 0 on error
#[no_mangle]
pub extern "C" fn lolite_init(use_same_process: bool) -> EngineHandle {
    let engine_mode = if use_same_process {
        EngineMode::SameProcess(Box::new(CssEngine::new()))
    } else {
        match worker_instance::WorkerInstance::new() {
            Ok(worker) => EngineMode::WorkerProcess(worker),
            Err(e) => {
                eprintln!("Failed to create worker instance: {}", e);
                return 0;
            }
        }
    };

    // Get next handle and store the engine
    let handle = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);

    let mut instances = ENGINE_INSTANCES.lock().unwrap();
    instances.insert(handle, engine_mode);

    handle
}

/// Add a CSS stylesheet to the engine
///
/// # Arguments
/// * `handle` - Engine handle returned from lolite_init
/// * `css_content` - Null-terminated CSS string
///
/// # Returns
/// * 0 on success, -1 on error
#[no_mangle]
pub extern "C" fn lolite_add_stylesheet(handle: EngineHandle, css_content: *const c_char) {
    if handle == 0 {
        eprintln!("Invalid engine handle");
        return;
    }

    if css_content.is_null() {
        eprintln!("CSS content is null");
        return;
    }

    let css_str = match unsafe { CStr::from_ptr(css_content) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Invalid UTF-8 in CSS content: {}", e);
            return;
        }
    };

    let instances = ENGINE_INSTANCES.lock().unwrap();
    match instances.get(&handle) {
        Some(EngineMode::SameProcess(engine)) => {
            engine.add_stylesheet(css_str);
        }
        Some(EngineMode::WorkerProcess(_worker)) => {
            // TODO: Implement worker process stylesheet addition
            eprintln!("Worker process mode not yet implemented for add_stylesheet");
        }
        None => {
            eprintln!("Engine handle not found");
        }
    }
}

/// Create a new document node
///
/// # Arguments
/// * `handle` - Engine handle returned from lolite_init
/// * `text_content` - Optional null-terminated text content (can be null)
///
/// # Returns
/// * Node ID on success, 0 on error (since root is always ID 0, we can distinguish)
#[no_mangle]
pub extern "C" fn lolite_create_node(handle: EngineHandle, text_content: *const c_char) -> u64 {
    if handle == 0 {
        eprintln!("Invalid engine handle");
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

    let instances = ENGINE_INSTANCES.lock().unwrap();
    match instances.get(&handle) {
        Some(EngineMode::SameProcess(engine)) => {
            let id = engine.create_node(text);
            id.as_u64()
        }
        Some(EngineMode::WorkerProcess(_worker)) => {
            // TODO: Implement worker process node creation
            eprintln!("Worker process mode not yet implemented for create_node");
            0
        }
        None => {
            eprintln!("Engine handle not found");
            0
        }
    }
}

/// Set parent-child relationship between nodes
///
/// # Arguments
/// * `handle` - Engine handle returned from lolite_init
/// * `parent_id` - ID of the parent node
/// * `child_id` - ID of the child node
///
/// # Returns
/// * 0 on success, -1 on error
#[no_mangle]
pub extern "C" fn lolite_set_parent(handle: EngineHandle, parent_id: u64, child_id: u64) {
    if handle == 0 {
        eprintln!("Invalid engine handle");
        return;
    }

    let parent = Id::from_u64(parent_id);
    let child = Id::from_u64(child_id);

    let instances = ENGINE_INSTANCES.lock().unwrap();
    match instances.get(&handle) {
        Some(EngineMode::SameProcess(engine)) => {
            engine.set_parent(parent, child);
        }
        Some(EngineMode::WorkerProcess(_worker)) => {
            // TODO: Implement worker process set_parent
            eprintln!("Worker process mode not yet implemented for set_parent");
        }
        None => {
            eprintln!("Engine handle not found");
        }
    }
}

/// Set an attribute on a node
///
/// # Arguments
/// * `handle` - Engine handle returned from lolite_init
/// * `node_id` - ID of the node
/// * `key` - Null-terminated attribute key string
/// * `value` - Null-terminated attribute value string
///
/// # Returns
/// * 0 on success, -1 on error
#[no_mangle]
pub extern "C" fn lolite_set_attribute(
    handle: EngineHandle,
    node_id: u64,
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

    let node = Id::from_u64(node_id);

    let instances = ENGINE_INSTANCES.lock().unwrap();
    match instances.get(&handle) {
        Some(EngineMode::SameProcess(engine)) => {
            engine.set_attribute(node, key_str, value_str);
        }
        Some(EngineMode::WorkerProcess(_worker)) => {
            // TODO: Implement worker process set_attribute
            eprintln!("Worker process mode not yet implemented for set_attribute");
        }
        None => {
            eprintln!("Engine handle not found");
        }
    }
}

/// Get the root node ID of the document
///
/// # Arguments
/// * `handle` - Engine handle returned from lolite_init
///
/// # Returns
/// * Root node ID (always 0 for the document root), or 0 if handle is invalid
#[no_mangle]
pub extern "C" fn lolite_root_id(handle: EngineHandle) -> u64 {
    if handle == 0 {
        eprintln!("Invalid engine handle");
        return 0;
    }

    let instances = ENGINE_INSTANCES.lock().unwrap();
    match instances.get(&handle) {
        Some(EngineMode::SameProcess(engine)) => engine.root_id().as_u64(),
        Some(EngineMode::WorkerProcess(_worker)) => {
            // TODO: Implement worker process root_id
            eprintln!("Worker process mode not yet implemented for root_id");
            0
        }
        None => {
            eprintln!("Engine handle not found");
            0
        }
    }
}

/// Cleanup and destroy an engine instance
///
/// # Arguments
/// * `handle` - Engine handle returned from lolite_init
///
/// # Returns
/// * 0 on success, -1 on error
#[no_mangle]
pub extern "C" fn lolite_destroy(handle: EngineHandle) -> c_int {
    if handle == 0 {
        eprintln!("Invalid engine handle");
        return -1;
    }

    let mut instances = ENGINE_INSTANCES.lock().unwrap();
    match instances.remove(&handle) {
        Some(_) => 0,
        None => {
            eprintln!("Engine handle not found");
            -1
        }
    }
}
