use ipc_channel::ipc;
use libloading::Library;
use sonate_common::WorkerRequest;
use std::collections::HashMap;
use std::env;
use std::ffi::CString;
use std::os::raw::{c_char, c_void};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

type EngineHandle = usize;

type SonateInitInternal = unsafe extern "C" fn(EngineHandle);
type SonateAddStylesheet = unsafe extern "C" fn(EngineHandle, *const c_char);
type SonateCreateNode = unsafe extern "C" fn(EngineHandle, u64, *const c_char) -> u64;
type SonateDestroyNode = unsafe extern "C" fn(EngineHandle, u64);
type SonateSetParent = unsafe extern "C" fn(EngineHandle, u64, u64);
type SonateSetAttribute = unsafe extern "C" fn(EngineHandle, u64, *const c_char, *const c_char);
type SonateRootId = unsafe extern "C" fn(EngineHandle) -> u64;
type SonateSetEventCallback = unsafe extern "C" fn(
    EngineHandle,
    Option<extern "C" fn(EngineHandle, *const SonateEvent, *mut c_void)>,
    *mut c_void,
) -> i32;
type SonateRun = unsafe extern "C" fn(EngineHandle) -> i32;
type SonateDestroy = unsafe extern "C" fn(EngineHandle) -> i32;

#[repr(C)]
#[derive(Clone, Copy)]
#[allow(dead_code)]
enum SonateEventType {
    Click = 1,
    Scroll = 2,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SonateEvent {
    event_type: SonateEventType,
    x: f64,
    y: f64,
    scroll_target_id: u64,
    scroll_dx: f64,
    scroll_dy: f64,
    element_ids: *const u64,
    element_count: usize,
}

static EVENT_SENDERS: LazyLock<
    Mutex<HashMap<EngineHandle, ipc::IpcSender<sonate_common::WorkerEvent>>>,
> = LazyLock::new(|| Mutex::new(HashMap::new()));

extern "C" fn forward_worker_event(
    handle: EngineHandle,
    event: *const SonateEvent,
    _user_data: *mut c_void,
) {
    if event.is_null() {
        return;
    }

    let sender = {
        let senders = EVENT_SENDERS.lock().unwrap();
        senders.get(&handle).cloned()
    };

    let Some(sender) = sender else {
        return;
    };

    let event = unsafe { &*event };
    let element_ids = if event.element_ids.is_null() || event.element_count == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(event.element_ids, event.element_count) }.to_vec()
    };

    let event_type = match event.event_type {
        SonateEventType::Click => sonate_common::WorkerEventType::Click,
        SonateEventType::Scroll => sonate_common::WorkerEventType::Scroll,
    };

    let _ = sender.send(sonate_common::WorkerEvent {
        event_type,
        x: event.x,
        y: event.y,
        scroll_target_id: event.scroll_target_id,
        scroll_dx: event.scroll_dx,
        scroll_dy: event.scroll_dy,
        element_ids,
    });
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // args[0] = exe
    // args[1] = method
    // args[2] = connection_key (ipc one-shot server name)
    let method = args.get(1).map(|s| s.as_str()).unwrap_or("");
    let connection_key = args.get(2).map(|s| s.as_str()).unwrap_or("");

    if method != "ipc_channel" {
        eprintln!("worker: unsupported method '{method}'");
        std::process::exit(2);
    }

    if connection_key.is_empty() {
        eprintln!("worker: missing connection key");
        std::process::exit(2);
    }

    // Connect to the host's one-shot server and send back a channel sender.
    let bootstrap = ipc::IpcSender::connect(connection_key.to_string())
        .expect("worker: failed to connect to host");
    let (tx, rx) = ipc::channel::<WorkerRequest>().expect("worker: failed to create channel");
    bootstrap
        .send(tx)
        .expect("worker: failed to send channel sender to host");

    // Load sonate dynamic library once and keep it alive for the whole process.
    let lib_path = resolve_library_path();
    let lib = unsafe {
        Library::new(&lib_path).unwrap_or_else(|e| {
            eprintln!("worker: failed to load sonate library at {lib_path:?}: {e}");
            std::process::exit(3);
        })
    };

    unsafe {
        let sonate_init_internal: libloading::Symbol<SonateInitInternal> = lib
            .get(b"sonate_init_internal\0")
            .expect("worker: missing symbol sonate_init_internal");
        let sonate_add_stylesheet: libloading::Symbol<SonateAddStylesheet> = lib
            .get(b"sonate_add_stylesheet\0")
            .expect("worker: missing symbol sonate_add_stylesheet");
        let sonate_create_node: libloading::Symbol<SonateCreateNode> = lib
            .get(b"sonate_create_node\0")
            .expect("worker: missing symbol sonate_create_node");
        let sonate_destroy_node: libloading::Symbol<SonateDestroyNode> = lib
            .get(b"sonate_destroy_node\0")
            .expect("worker: missing symbol sonate_destroy_node");
        let sonate_set_parent: libloading::Symbol<SonateSetParent> = lib
            .get(b"sonate_set_parent\0")
            .expect("worker: missing symbol sonate_set_parent");
        let sonate_set_attribute: libloading::Symbol<SonateSetAttribute> = lib
            .get(b"sonate_set_attribute\0")
            .expect("worker: missing symbol sonate_set_attribute");
        let sonate_root_id: libloading::Symbol<SonateRootId> = lib
            .get(b"sonate_root_id\0")
            .expect("worker: missing symbol sonate_root_id");
        let sonate_set_event_callback: libloading::Symbol<SonateSetEventCallback> = lib
            .get(b"sonate_set_event_callback\0")
            .expect("worker: missing symbol sonate_set_event_callback");
        let sonate_run: libloading::Symbol<SonateRun> = lib
            .get(b"sonate_run\0")
            .expect("worker: missing symbol sonate_run");
        let sonate_destroy: libloading::Symbol<SonateDestroy> = lib
            .get(b"sonate_destroy\0")
            .expect("worker: missing symbol sonate_destroy");

        loop {
            let msg = match rx.recv() {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("worker: ipc receive error: {e}");
                    break;
                }
            };

            match msg {
                WorkerRequest::InitInternal { handle } => {
                    sonate_init_internal(handle as EngineHandle);
                }
                WorkerRequest::AddStylesheet { handle, css } => match CString::new(css) {
                    Ok(c_css) => {
                        sonate_add_stylesheet(handle as EngineHandle, c_css.as_ptr());
                    }
                    Err(_) => {
                        eprintln!("worker: stylesheet contains interior NUL byte");
                    }
                },
                WorkerRequest::CreateNode {
                    handle,
                    node_id,
                    text,
                } => {
                    match text {
                        None => {
                            let _ = sonate_create_node(
                                handle as EngineHandle,
                                node_id,
                                std::ptr::null(),
                            );
                        }
                        Some(s) => match CString::new(s) {
                            Ok(c_text) => {
                                let _ = sonate_create_node(
                                    handle as EngineHandle,
                                    node_id,
                                    c_text.as_ptr(),
                                );
                            }
                            Err(_) => {
                                eprintln!("worker: text content contains interior NUL byte");
                            }
                        },
                    };
                }
                WorkerRequest::DestroyNode { handle, node_id } => {
                    sonate_destroy_node(handle as EngineHandle, node_id);
                }
                WorkerRequest::SetParent {
                    handle,
                    parent_id,
                    child_id,
                } => {
                    sonate_set_parent(handle as EngineHandle, parent_id, child_id);
                }
                WorkerRequest::SetAttribute {
                    handle,
                    node_id,
                    key,
                    value,
                } => {
                    let c_key = match CString::new(key) {
                        Ok(s) => s,
                        Err(_) => {
                            eprintln!("worker: attribute key contains interior NUL byte");
                            continue;
                        }
                    };
                    let c_value = match CString::new(value) {
                        Ok(s) => s,
                        Err(_) => {
                            eprintln!("worker: attribute value contains interior NUL byte");
                            continue;
                        }
                    };

                    sonate_set_attribute(
                        handle as EngineHandle,
                        node_id,
                        c_key.as_ptr(),
                        c_value.as_ptr(),
                    );
                }
                WorkerRequest::RootId { handle, reply_to } => {
                    let id = sonate_root_id(handle as EngineHandle);
                    let _ = reply_to.send(id);
                }
                WorkerRequest::SetEventSender { handle, sender } => {
                    EVENT_SENDERS
                        .lock()
                        .unwrap()
                        .insert(handle as EngineHandle, sender);

                    let code = sonate_set_event_callback(
                        handle as EngineHandle,
                        Some(forward_worker_event),
                        std::ptr::null_mut(),
                    );
                    if code != 0 {
                        eprintln!("worker: failed to register event callback for handle {handle}");
                    }
                }
                WorkerRequest::Run { handle, reply_to } => {
                    let code = sonate_run(handle as EngineHandle);
                    let _ = reply_to.send(code);
                }
                WorkerRequest::Destroy { handle, reply_to } => {
                    let code = sonate_destroy(handle as EngineHandle);
                    EVENT_SENDERS
                        .lock()
                        .unwrap()
                        .remove(&(handle as EngineHandle));
                    let _ = reply_to.send(code);
                }
                WorkerRequest::Shutdown => {
                    EVENT_SENDERS.lock().unwrap().clear();
                    break;
                }
            }
        }
    }
}

fn resolve_library_path() -> PathBuf {
    if let Ok(path) = std::env::var("SONATE_LIBRARY_PATH") {
        return PathBuf::from(path);
    }

    let name = default_library_name();

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            let candidate = dir.join(&name);
            if candidate.exists() {
                return candidate;
            }
        }
    }

    Path::new(&name).to_path_buf()
}

fn default_library_name() -> String {
    if cfg!(target_os = "windows") {
        "sonate.dll".to_string()
    } else if cfg!(target_os = "macos") {
        "libsonate.dylib".to_string()
    } else {
        "libsonate.so".to_string()
    }
}
