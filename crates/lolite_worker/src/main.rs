use ipc_channel::ipc;
use libloading::Library;
use lolite_common::WorkerRequest;
use std::env;
use std::ffi::CString;
use std::os::raw::c_char;
use std::path::{Path, PathBuf};

type EngineHandle = usize;

type LoliteInitInternal = unsafe extern "C" fn(EngineHandle);
type LoliteAddStylesheet = unsafe extern "C" fn(EngineHandle, *const c_char);
type LoliteCreateNode = unsafe extern "C" fn(EngineHandle, *const c_char) -> u64;
type LoliteSetParent = unsafe extern "C" fn(EngineHandle, u64, u64);
type LoliteSetAttribute = unsafe extern "C" fn(EngineHandle, u64, *const c_char, *const c_char);
type LoliteRootId = unsafe extern "C" fn(EngineHandle) -> u64;
type LoliteRun = unsafe extern "C" fn(EngineHandle) -> i32;
type LoliteDestroy = unsafe extern "C" fn(EngineHandle) -> i32;

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

    // Load lolite dynamic library once and keep it alive for the whole process.
    let lib_path = resolve_library_path();
    let lib = unsafe {
        Library::new(&lib_path).unwrap_or_else(|e| {
            eprintln!("worker: failed to load lolite library at {lib_path:?}: {e}");
            std::process::exit(3);
        })
    };

    unsafe {
        let lolite_init_internal: libloading::Symbol<LoliteInitInternal> = lib
            .get(b"lolite_init_internal\0")
            .expect("worker: missing symbol lolite_init_internal");
        let lolite_add_stylesheet: libloading::Symbol<LoliteAddStylesheet> = lib
            .get(b"lolite_add_stylesheet\0")
            .expect("worker: missing symbol lolite_add_stylesheet");
        let lolite_create_node: libloading::Symbol<LoliteCreateNode> = lib
            .get(b"lolite_create_node\0")
            .expect("worker: missing symbol lolite_create_node");
        let lolite_set_parent: libloading::Symbol<LoliteSetParent> = lib
            .get(b"lolite_set_parent\0")
            .expect("worker: missing symbol lolite_set_parent");
        let lolite_set_attribute: libloading::Symbol<LoliteSetAttribute> = lib
            .get(b"lolite_set_attribute\0")
            .expect("worker: missing symbol lolite_set_attribute");
        let lolite_root_id: libloading::Symbol<LoliteRootId> = lib
            .get(b"lolite_root_id\0")
            .expect("worker: missing symbol lolite_root_id");
        let lolite_run: libloading::Symbol<LoliteRun> = lib
            .get(b"lolite_run\0")
            .expect("worker: missing symbol lolite_run");
        let lolite_destroy: libloading::Symbol<LoliteDestroy> = lib
            .get(b"lolite_destroy\0")
            .expect("worker: missing symbol lolite_destroy");

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
                    lolite_init_internal(handle as EngineHandle);
                }
                WorkerRequest::AddStylesheet { handle, css } => match CString::new(css) {
                    Ok(c_css) => {
                        lolite_add_stylesheet(handle as EngineHandle, c_css.as_ptr());
                    }
                    Err(_) => {
                        eprintln!("worker: stylesheet contains interior NUL byte");
                    }
                },
                WorkerRequest::CreateNode {
                    handle,
                    text,
                    reply_to,
                } => {
                    let id = match text {
                        None => lolite_create_node(handle as EngineHandle, std::ptr::null()),
                        Some(s) => match CString::new(s) {
                            Ok(c_text) => {
                                lolite_create_node(handle as EngineHandle, c_text.as_ptr())
                            }
                            Err(_) => {
                                eprintln!("worker: text content contains interior NUL byte");
                                0
                            }
                        },
                    };

                    let _ = reply_to.send(id);
                }
                WorkerRequest::SetParent {
                    handle,
                    parent_id,
                    child_id,
                } => {
                    lolite_set_parent(handle as EngineHandle, parent_id, child_id);
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

                    lolite_set_attribute(
                        handle as EngineHandle,
                        node_id,
                        c_key.as_ptr(),
                        c_value.as_ptr(),
                    );
                }
                WorkerRequest::RootId { handle, reply_to } => {
                    let id = lolite_root_id(handle as EngineHandle);
                    let _ = reply_to.send(id);
                }
                WorkerRequest::Run { handle, reply_to } => {
                    let code = lolite_run(handle as EngineHandle);
                    let _ = reply_to.send(code);
                }
                WorkerRequest::Destroy { handle, reply_to } => {
                    let code = lolite_destroy(handle as EngineHandle);
                    let _ = reply_to.send(code);
                }
                WorkerRequest::Shutdown => {
                    break;
                }
            }
        }
    }
}

fn resolve_library_path() -> PathBuf {
    if let Ok(path) = std::env::var("LOLITE_LIBRARY_PATH") {
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
        "lolite.dll".to_string()
    } else if cfg!(target_os = "macos") {
        "liblolite.dylib".to_string()
    } else {
        "liblolite.so".to_string()
    }
}
