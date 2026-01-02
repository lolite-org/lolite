use crate::EngineHandle;
use ipc_channel::ipc::{self, IpcOneShotServer, IpcSender};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[cfg(windows)]
const WORKER_FILE: &str = "lolite_worker.exe";
#[cfg(not(windows))]
const WORKER_FILE: &str = "lolite_worker";

pub struct WorkerInstance {
    #[allow(dead_code)]
    process: std::process::Child,
    sender: IpcSender<lolite_common::WorkerRequest>,
}

impl WorkerInstance {
    pub fn new() -> std::io::Result<WorkerInstance> {
        // Worker connects back and sends an IpcSender that we can use to send requests.
        let (server, server_name) =
            IpcOneShotServer::<IpcSender<lolite_common::WorkerRequest>>::new()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        let process = spawn_worker("ipc_channel", &server_name)?;

        let (_rx, sender) = server
            .accept()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        Ok(WorkerInstance { process, sender })
    }

    pub fn init(&self, handle: EngineHandle) {
        if let Err(e) = self
            .sender
            .send(lolite_common::WorkerRequest::InitInternal {
                handle: handle as u64,
            })
        {
            eprintln!("Failed to send InitInternal to worker: {e}");
        }
    }

    pub fn add_stylesheet(&self, handle: EngineHandle, css_content: *const c_char) {
        if css_content.is_null() {
            eprintln!("CSS content is null");
            return;
        }

        let css_str = match unsafe { CStr::from_ptr(css_content) }.to_str() {
            Ok(s) => s.to_string(),
            Err(e) => {
                eprintln!("Invalid UTF-8 in CSS content: {e}");
                return;
            }
        };

        if let Err(e) = self
            .sender
            .send(lolite_common::WorkerRequest::AddStylesheet {
                handle: handle as u64,
                css: css_str,
            })
        {
            eprintln!("Failed to send AddStylesheet to worker: {e}");
        }
    }

    pub fn create_node(&self, handle: EngineHandle, text_content: *const c_char) -> u64 {
        let text = if text_content.is_null() {
            None
        } else {
            match unsafe { CStr::from_ptr(text_content) }.to_str() {
                Ok(s) => Some(s.to_string()),
                Err(e) => {
                    eprintln!("Invalid UTF-8 in text content: {e}");
                    return 0;
                }
            }
        };

        let (reply_tx, reply_rx) = match ipc::channel::<u64>() {
            Ok(ch) => ch,
            Err(e) => {
                eprintln!("Failed to create reply channel: {e}");
                return 0;
            }
        };

        if let Err(e) = self.sender.send(lolite_common::WorkerRequest::CreateNode {
            handle: handle as u64,
            text,
            reply_to: reply_tx,
        }) {
            eprintln!("Failed to send CreateNode to worker: {e}");
            return 0;
        }

        match reply_rx.recv() {
            Ok(id) => id,
            Err(e) => {
                eprintln!("Failed to receive CreateNode response: {e}");
                0
            }
        }
    }

    pub fn set_parent(&self, handle: EngineHandle, parent_id: u64, child_id: u64) {
        if let Err(e) = self.sender.send(lolite_common::WorkerRequest::SetParent {
            handle: handle as u64,
            parent_id,
            child_id,
        }) {
            eprintln!("Failed to send SetParent to worker: {e}");
        }
    }

    pub fn set_attribute(
        &self,
        handle: EngineHandle,
        node_id: u64,
        key: *const c_char,
        value: *const c_char,
    ) {
        if key.is_null() || value.is_null() {
            eprintln!("Key or value is null");
            return;
        }

        let key_str = match unsafe { CStr::from_ptr(key) }.to_str() {
            Ok(s) => s.to_string(),
            Err(e) => {
                eprintln!("Invalid UTF-8 in attribute key: {e}");
                return;
            }
        };

        let value_str = match unsafe { CStr::from_ptr(value) }.to_str() {
            Ok(s) => s.to_string(),
            Err(e) => {
                eprintln!("Invalid UTF-8 in attribute value: {e}");
                return;
            }
        };

        if let Err(e) = self
            .sender
            .send(lolite_common::WorkerRequest::SetAttribute {
                handle: handle as u64,
                node_id,
                key: key_str,
                value: value_str,
            })
        {
            eprintln!("Failed to send SetAttribute to worker: {e}");
        }
    }

    pub fn root_id(&self, handle: EngineHandle) -> u64 {
        let (reply_tx, reply_rx) = match ipc::channel::<u64>() {
            Ok(ch) => ch,
            Err(e) => {
                eprintln!("Failed to create reply channel: {e}");
                return 0;
            }
        };

        if let Err(e) = self.sender.send(lolite_common::WorkerRequest::RootId {
            handle: handle as u64,
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

    pub fn run(&self, handle: EngineHandle) -> i32 {
        let (reply_tx, reply_rx) = match ipc::channel::<i32>() {
            Ok(ch) => ch,
            Err(e) => {
                eprintln!("Failed to create reply channel: {e}");
                return -1;
            }
        };

        if let Err(e) = self.sender.send(lolite_common::WorkerRequest::Run {
            handle: handle as u64,
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

    pub fn destroy_engine(&self, handle: EngineHandle) -> i32 {
        let (reply_tx, reply_rx) = match ipc::channel::<i32>() {
            Ok(ch) => ch,
            Err(e) => {
                eprintln!("Failed to create reply channel: {e}");
                return -1;
            }
        };

        if let Err(e) = self.sender.send(lolite_common::WorkerRequest::Destroy {
            handle: handle as u64,
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

impl Drop for WorkerInstance {
    fn drop(&mut self) {
        let _ = self.sender.send(lolite_common::WorkerRequest::Shutdown);
        let _ = self.process.kill();
    }
}

fn spawn_worker(method: &str, connection_key: &str) -> std::io::Result<std::process::Child> {
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
    if let Ok(path) = std::env::var("LOLITE_WORKER_PATH") {
        return Some(PathBuf::from(path));
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
