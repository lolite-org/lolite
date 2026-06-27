use crate::engine_backend::{EngineBackend, SonateId};
use ipc_channel::ipc::{self, IpcOneShotServer, IpcSender};
use std::os::raw::c_int;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

pub struct WorkerBackend {
    handle: usize,
    process: Child,
    sender: IpcSender<sonate_common::WorkerRequest>,
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
            process,
            sender,
        };

        backend.init_internal();
        Ok(backend)
    }

    fn init_internal(&self) {
        if let Err(e) = self
            .sender
            .send(sonate_common::WorkerRequest::InitInternal {
                handle: self.handle as u64,
            })
        {
            eprintln!("Failed to send InitInternal to worker: {e}");
        }
    }

    fn shutdown(&self) {
        let _ = self.sender.send(sonate_common::WorkerRequest::Shutdown);
    }
}

impl EngineBackend for WorkerBackend {
    fn add_stylesheet(&self, css: String) {
        if let Err(e) = self
            .sender
            .send(sonate_common::WorkerRequest::AddStylesheet {
                handle: self.handle as u64,
                css,
            })
        {
            eprintln!("Failed to send AddStylesheet to worker: {e}");
        }
    }

    fn create_node(&self, node_id: SonateId, text: Option<String>) {
        if let Err(e) = self.sender.send(sonate_common::WorkerRequest::CreateNode {
            handle: self.handle as u64,
            node_id,
            text,
        }) {
            eprintln!("Failed to send CreateNode to worker: {e}");
        }
    }

    fn destroy_node(&self, node_id: SonateId) {
        if let Err(e) = self.sender.send(sonate_common::WorkerRequest::DestroyNode {
            handle: self.handle as u64,
            node_id,
        }) {
            eprintln!("Failed to send DestroyNode to worker: {e}");
        }
    }

    fn set_parent(&self, parent_id: SonateId, child_id: SonateId) {
        if let Err(e) = self.sender.send(sonate_common::WorkerRequest::SetParent {
            handle: self.handle as u64,
            parent_id,
            child_id,
        }) {
            eprintln!("Failed to send SetParent to worker: {e}");
        }
    }

    fn set_attribute(&self, node_id: SonateId, key: String, value: String) {
        if let Err(e) = self
            .sender
            .send(sonate_common::WorkerRequest::SetAttribute {
                handle: self.handle as u64,
                node_id,
                key,
                value,
            })
        {
            eprintln!("Failed to send SetAttribute to worker: {e}");
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

        if let Err(e) = self.sender.send(sonate_common::WorkerRequest::RootId {
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

    fn run(&self) -> c_int {
        let (reply_tx, reply_rx) = match ipc::channel::<i32>() {
            Ok(ch) => ch,
            Err(e) => {
                eprintln!("Failed to create reply channel: {e}");
                return -1;
            }
        };

        if let Err(e) = self.sender.send(sonate_common::WorkerRequest::Run {
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

        if let Err(e) = self.sender.send(sonate_common::WorkerRequest::Destroy {
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
        let _ = self.process.kill();
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
