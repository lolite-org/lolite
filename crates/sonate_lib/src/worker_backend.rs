use crate::engine_backend::{EngineBackend, SonateEventCallback, SonateId};
use ipc_channel::ipc::{self, IpcOneShotServer, IpcSender};
use std::ffi::c_void;
use std::os::raw::c_int;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::ptr;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Default)]
struct EventCallbackState {
    callback: SonateEventCallback,
    user_data: usize,
}

pub struct WorkerBackend {
    handle: usize,
    process: Mutex<Child>,
    sender: Mutex<IpcSender<sonate_common::WorkerRequest>>,
    callback_state: Arc<Mutex<EventCallbackState>>,
    event_listener_thread: Mutex<Option<thread::JoinHandle<()>>>,
}

impl WorkerBackend {
    pub fn new(handle: usize) -> std::io::Result<Self> {
        // Worker connects back and sends an IpcSender that we can use to send requests.
        let (server, server_name) =
            IpcOneShotServer::<IpcSender<sonate_common::WorkerRequest>>::new()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        let process = spawn_worker("ipc_channel", &server_name)?;

        let (_rx, sender) = server
            .accept()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        let backend = Self {
            handle,
            process: Mutex::new(process),
            sender: Mutex::new(sender),
            callback_state: Arc::new(Mutex::new(EventCallbackState::default())),
            event_listener_thread: Mutex::new(None),
        };

        backend.init_internal();
        backend.init_event_bridge()?;
        Ok(backend)
    }

    fn send(&self, request: sonate_common::WorkerRequest) -> Result<(), String> {
        self.sender
            .lock()
            .unwrap()
            .send(request)
            .map_err(|e| e.to_string())
    }

    fn init_internal(&self) {
        if let Err(e) = self.send(sonate_common::WorkerRequest::InitInternal {
            handle: self.handle as u64,
        }) {
            eprintln!("Failed to send InitInternal to worker: {e}");
        }
    }

    fn shutdown(&self) {
        let _ = self.send(sonate_common::WorkerRequest::Shutdown);
    }

    fn init_event_bridge(&self) -> std::io::Result<()> {
        let (event_tx, event_rx) = ipc::channel::<sonate_common::WorkerEvent>()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        self.send(sonate_common::WorkerRequest::SetEventSender {
            handle: self.handle as u64,
            sender: event_tx,
        })
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        let callback_state = Arc::clone(&self.callback_state);
        let handle = self.handle;
        *self.event_listener_thread.lock().unwrap() = Some(thread::spawn(move || {
            while let Ok(worker_event) = event_rx.recv() {
                let (callback, user_data) = {
                    let state = callback_state.lock().unwrap();
                    (state.callback, state.user_data)
                };

                let Some(callback) = callback else {
                    continue;
                };

                let event_type = match worker_event.event_type {
                    sonate_common::WorkerEventType::Click => crate::SonateEventType::Click,
                    sonate_common::WorkerEventType::Scroll => crate::SonateEventType::Scroll,
                };

                let element_ids = worker_event.element_ids;
                let event = crate::SonateEvent {
                    event_type,
                    x: worker_event.x,
                    y: worker_event.y,
                    scroll_target_id: worker_event.scroll_target_id,
                    scroll_dx: worker_event.scroll_dx,
                    scroll_dy: worker_event.scroll_dy,
                    element_ids: if element_ids.is_empty() {
                        ptr::null()
                    } else {
                        element_ids.as_ptr()
                    },
                    element_count: element_ids.len(),
                };

                callback(handle, &event, user_data as *mut c_void);
            }
        }));

        Ok(())
    }
}

impl EngineBackend for WorkerBackend {
    fn add_stylesheet(&self, css: String) {
        if let Err(e) = self.send(sonate_common::WorkerRequest::AddStylesheet {
            handle: self.handle as u64,
            css,
        }) {
            eprintln!("Failed to send AddStylesheet to worker: {e}");
        }
    }

    fn create_node(&self, node_id: SonateId, text: Option<String>) {
        if let Err(e) = self.send(sonate_common::WorkerRequest::CreateNode {
            handle: self.handle as u64,
            node_id,
            text,
        }) {
            eprintln!("Failed to send CreateNode to worker: {e}");
        }
    }

    fn destroy_node(&self, node_id: SonateId) {
        if let Err(e) = self.send(sonate_common::WorkerRequest::DestroyNode {
            handle: self.handle as u64,
            node_id,
        }) {
            eprintln!("Failed to send DestroyNode to worker: {e}");
        }
    }

    fn set_parent(&self, parent_id: SonateId, child_id: SonateId) {
        if let Err(e) = self.send(sonate_common::WorkerRequest::SetParent {
            handle: self.handle as u64,
            parent_id,
            child_id,
        }) {
            eprintln!("Failed to send SetParent to worker: {e}");
        }
    }

    fn set_attribute(&self, node_id: SonateId, key: String, value: String) {
        if let Err(e) = self.send(sonate_common::WorkerRequest::SetAttribute {
            handle: self.handle as u64,
            node_id,
            key,
            value,
        }) {
            eprintln!("Failed to send SetAttribute to worker: {e}");
        }
    }

    fn set_text(&self, node_id: SonateId, text: Option<String>) {
        if let Err(e) = self.send(sonate_common::WorkerRequest::SetText {
            handle: self.handle as u64,
            node_id,
            text,
        }) {
            eprintln!("Failed to send SetText to worker: {e}");
        }
    }

    fn root_id(&self) -> SonateId {
        let (reply_tx, reply_rx) = match ipc::channel::<u64>() {
            Ok(ch) => ch,
            Err(e) => {
                eprintln!("Failed to create reply channel: {e}");
                return 0;
            }
        };

        if let Err(e) = self.send(sonate_common::WorkerRequest::RootId {
            handle: self.handle as u64,
            reply_to: reply_tx,
        }) {
            eprintln!("Failed to send RootId to worker: {e}");
            return 0;
        }

        match reply_rx.recv() {
            Ok(id) => id,
            Err(e) => {
                eprintln!("Failed to receive RootId response: {e}");
                0
            }
        }
    }

    fn set_event_callback(&self, callback: SonateEventCallback, user_data: *mut c_void) -> i32 {
        let mut state = self.callback_state.lock().unwrap();
        state.callback = callback;
        state.user_data = user_data as usize;
        0
    }

    fn run(&self) -> c_int {
        let (reply_tx, reply_rx) = match ipc::channel::<i32>() {
            Ok(ch) => ch,
            Err(e) => {
                eprintln!("Failed to create reply channel: {e}");
                return -1;
            }
        };

        if let Err(e) = self.send(sonate_common::WorkerRequest::Run {
            handle: self.handle as u64,
            reply_to: reply_tx,
        }) {
            eprintln!("Failed to send Run to worker: {e}");
            return -1;
        }

        match reply_rx.recv() {
            Ok(code) => code,
            Err(e) => {
                eprintln!("Failed to receive Run response: {e}");
                -1
            }
        }
    }

    fn destroy(&self) -> c_int {
        let (reply_tx, reply_rx) = match ipc::channel::<i32>() {
            Ok(ch) => ch,
            Err(e) => {
                eprintln!("Failed to create reply channel: {e}");
                return -1;
            }
        };

        if let Err(e) = self.send(sonate_common::WorkerRequest::Destroy {
            handle: self.handle as u64,
            reply_to: reply_tx,
        }) {
            eprintln!("Failed to send Destroy to worker: {e}");
            return -1;
        }

        match reply_rx.recv() {
            Ok(code) => code,
            Err(e) => {
                eprintln!("Failed to receive Destroy response: {e}");
                -1
            }
        }
    }
}

impl Drop for WorkerBackend {
    fn drop(&mut self) {
        self.shutdown();
        let _ = self.process.lock().unwrap().kill();
        if let Some(thread) = self.event_listener_thread.lock().unwrap().take() {
            let _ = thread.join();
        }
    }
}

#[cfg(windows)]
const WORKER_FILE: &str = "sonate_worker.exe";
#[cfg(not(windows))]
const WORKER_FILE: &str = "sonate_worker";

fn spawn_worker(method: &str, connection_key: &str) -> std::io::Result<Child> {
    let worker_path = resolve_worker_path().expect("Failed to resolve worker path");

    println!("Running worker at {worker_path:?}");

    Command::new(worker_path)
        .arg(method)
        .arg(connection_key)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
}

fn resolve_worker_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("SONATE_WORKER_PATH") {
        return Some(PathBuf::from(path));
    }

    // Look next to the loaded sonate dynamic library. This is the common case
    // when the library is embedded by another runtime (e.g. Deno) whose
    // executable does not live next to the worker binary.
    if let Some(dir) = current_library_dir() {
        let candidate = dir.join(WORKER_FILE);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            let candidate = dir.join(WORKER_FILE);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    // We do not do PATH lookup, so we return None
    None
}

/// Directory containing the sonate dynamic library this code is compiled into.
#[cfg(unix)]
fn current_library_dir() -> Option<PathBuf> {
    let mut info: libc::Dl_info = unsafe { std::mem::zeroed() };
    let symbol_in_this_library = resolve_worker_path as *const c_void;

    let found = unsafe { libc::dladdr(symbol_in_this_library, &mut info) };
    if found == 0 || info.dli_fname.is_null() {
        return None;
    }

    let path = unsafe { std::ffi::CStr::from_ptr(info.dli_fname) };
    let path = PathBuf::from(path.to_string_lossy().into_owned());
    path.parent().map(|dir| dir.to_path_buf())
}

#[cfg(not(unix))]
fn current_library_dir() -> Option<PathBuf> {
    None
}
