use lolite::{Engine, Id, Params};
use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

mod worker_instance;

/// Engine wrapper that can operate in two modes:
/// - Same process: Direct CssEngine usage
/// - Worker process: Communication through WorkerInstance
enum EngineMode {
    SameProcess(Box<Engine>),
    WorkerProcess(Arc<Mutex<worker_instance::WorkerInstance>>),
}

struct EngineEntry {
    mode: EngineMode,
    // 0 is reserved for the document root
    next_node_id: AtomicU64,
}

/// Handle type for engine instances
pub type EngineHandle = usize;

/// ID type for nodes and other engine-owned objects.
pub type LoliteId = u64;

/// Thread-safe global storage for engine instances
/// We use Box<CssEngine> to store on heap and avoid Send+Sync requirements for static storage
/// Note: While CssEngine has internal synchronization via FairMutex, its inner types (Rc<RefCell<>>)
/// are not Send+Sync, so we need to be careful about cross-thread access.
/// Each engine should be accessed from the thread that created it, or proper synchronization
/// should be ensured by the caller.
static ENGINE_INSTANCES: std::sync::LazyLock<Mutex<HashMap<EngineHandle, EngineEntry>>> =
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
    let handle = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);

    if use_same_process {
        ENGINE_INSTANCES.lock().unwrap().insert(
            handle,
            EngineEntry {
                mode: EngineMode::SameProcess(Box::new(Engine::new())),
                next_node_id: AtomicU64::new(1),
            },
        );
    } else {
        let worker = match worker_instance::WorkerInstance::new() {
            Ok(worker) => worker,
            Err(e) => {
                eprintln!("Failed to create worker instance: {}", e);
                return 0;
            }
        };

        let worker = Arc::new(Mutex::new(worker));

        worker.lock().unwrap().init(handle);

        ENGINE_INSTANCES.lock().unwrap().insert(
            handle,
            EngineEntry {
                mode: EngineMode::WorkerProcess(worker),
                next_node_id: AtomicU64::new(1),
            },
        );
    };

    handle
}

#[no_mangle]
pub extern "C" fn lolite_init_internal(handle: EngineHandle) {
    ENGINE_INSTANCES.lock().unwrap().insert(
        handle,
        EngineEntry {
            mode: EngineMode::SameProcess(Box::new(Engine::new())),
            next_node_id: AtomicU64::new(1),
        },
    );
}

/// Add a CSS stylesheet to the engine
///
/// # Arguments
/// * `handle` - Engine handle returned from lolite_init
/// * `css_content` - Null-terminated CSS string
#[no_mangle]
pub extern "C" fn lolite_add_stylesheet(handle: EngineHandle, css_content: *const c_char) {
    let instances = ENGINE_INSTANCES.lock().unwrap();
    match instances.get(&handle) {
        Some(entry) => match &entry.mode {
            EngineMode::SameProcess(engine) => {
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
                engine.add_stylesheet(css_str);
            }
            EngineMode::WorkerProcess(worker) => {
                worker.lock().unwrap().add_stylesheet(handle, css_content);
            }
        },
        None => eprintln!("Engine handle not found"),
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
pub extern "C" fn lolite_create_node(
    handle: EngineHandle,
    text_content: *const c_char,
) -> LoliteId {
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

    // IMPORTANT: do not hold the global lock while sending commands or waiting on a worker.
    let (node_id, engine, worker) = {
        let instances = ENGINE_INSTANCES.lock().unwrap();
        let Some(entry) = instances.get(&handle) else {
            eprintln!("Engine handle not found");
            return 0;
        };

        let node_id = entry.next_node_id.fetch_add(1, Ordering::Relaxed);

        match &entry.mode {
            EngineMode::SameProcess(engine) => (node_id, Some(engine.as_ref().clone()), None),
            EngineMode::WorkerProcess(worker) => (node_id, None, Some(Arc::clone(worker))),
        }
    };

    if let Some(engine) = engine {
        let _ = engine.create_node(Id::from_u64(node_id), text);
        return node_id;
    }

    let Some(worker) = worker else {
        eprintln!("Internal error: missing engine and worker");
        return 0;
    };

    worker.lock().unwrap().create_node(handle, node_id, text);
    node_id
}

/// Internal helper used by the worker process to create a node with a specific ID.
///
/// This lets the host generate IDs locally and send them over IPC, avoiding a blocking round-trip.
#[no_mangle]
pub extern "C" fn lolite_create_node_internal(
    handle: EngineHandle,
    node_id: LoliteId,
    text_content: *const c_char,
) {
    if handle == 0 {
        eprintln!("Invalid engine handle");
        return;
    }

    let text = if text_content.is_null() {
        None
    } else {
        match unsafe { CStr::from_ptr(text_content) }.to_str() {
            Ok(s) => Some(s.to_string()),
            Err(e) => {
                eprintln!("Invalid UTF-8 in text content: {}", e);
                return;
            }
        }
    };

    // Clone engine/worker out from under the lock.
    let (engine, worker) = {
        let instances = ENGINE_INSTANCES.lock().unwrap();
        let Some(entry) = instances.get(&handle) else {
            eprintln!("Engine handle not found");
            return;
        };

        match &entry.mode {
            EngineMode::SameProcess(engine) => (Some(engine.as_ref().clone()), None),
            EngineMode::WorkerProcess(worker) => (None, Some(Arc::clone(worker))),
        }
    };

    if let Some(engine) = engine {
        let _ = engine.create_node(Id::from_u64(node_id), text);
        return;
    }

    let Some(worker) = worker else {
        eprintln!("Internal error: missing engine and worker");
        return;
    };

    worker.lock().unwrap().create_node(handle, node_id, text);
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
pub extern "C" fn lolite_set_parent(handle: EngineHandle, parent_id: LoliteId, child_id: LoliteId) {
    if handle == 0 {
        eprintln!("Invalid engine handle");
        return;
    }

    let parent = Id::from_u64(parent_id);
    let child = Id::from_u64(child_id);

    let instances = ENGINE_INSTANCES.lock().unwrap();
    match instances.get(&handle) {
        Some(entry) => match &entry.mode {
            EngineMode::SameProcess(engine) => {
                engine.set_parent(parent, child);
            }
            EngineMode::WorkerProcess(worker) => {
                worker
                    .lock()
                    .unwrap()
                    .set_parent(handle, parent_id, child_id);
            }
        },
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
    node_id: LoliteId,
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
        Some(entry) => match &entry.mode {
            EngineMode::SameProcess(engine) => {
                engine.set_attribute(node, key_str, value_str);
            }
            EngineMode::WorkerProcess(worker) => {
                worker
                    .lock()
                    .unwrap()
                    .set_attribute(handle, node_id, key, value);
            }
        },
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
pub extern "C" fn lolite_root_id(handle: EngineHandle) -> LoliteId {
    if handle == 0 {
        eprintln!("Invalid engine handle");
        return 0;
    }

    let instances = ENGINE_INSTANCES.lock().unwrap();
    match instances.get(&handle) {
        Some(entry) => match &entry.mode {
            EngineMode::SameProcess(engine) => engine.root_id().as_u64(),
            EngineMode::WorkerProcess(worker) => worker.lock().unwrap().root_id(handle),
        },
        None => {
            eprintln!("Engine handle not found");
            0
        }
    }
}

/// Run the engine event loop (blocking).
///
/// # Arguments
/// * `handle` - Engine handle returned from lolite_init
///
/// # Returns
/// * 0 on success, -1 on error
#[no_mangle]
pub extern "C" fn lolite_run(handle: EngineHandle) -> c_int {
    if handle == 0 {
        eprintln!("Invalid engine handle");
        return -1;
    }

    // IMPORTANT: do not hold the global lock while running the event loop.
    // - SameProcess: clone the Engine
    // - WorkerProcess: clone the Arc so we can call WorkerInstance::run
    let (engine, worker) = {
        let instances = ENGINE_INSTANCES.lock().unwrap();
        match instances.get(&handle) {
            Some(entry) => match &entry.mode {
                EngineMode::SameProcess(engine) => (Some(engine.as_ref().clone()), None),
                EngineMode::WorkerProcess(worker) => (None, Some(Arc::clone(worker))),
            },
            None => {
                eprintln!("Engine handle not found");
                return -1;
            }
        }
    };

    if let Some(engine) = engine {
        return match engine.run(Params { on_click: None }) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("lolite_run failed: {:?}", err);
                -1
            }
        };
    }

    let Some(worker) = worker else {
        eprintln!("Internal error: missing engine and worker");
        return -1;
    };

    let code = worker.lock().unwrap().run(handle);
    code
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
        Some(entry) => match entry.mode {
            EngineMode::SameProcess(_) => 0,
            EngineMode::WorkerProcess(worker) => worker.lock().unwrap().destroy_engine(handle),
        },
        None => {
            eprintln!("Engine handle not found");
            -1
        }
    }
}
