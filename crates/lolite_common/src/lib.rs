use ipc_channel::ipc::IpcSender;
use serde::{Deserialize, Serialize};

/// Cross-process requests sent from the host (lolite_lib) to the worker process (lolite_worker).
///
/// This is intentionally small and can be extended as more FFI functions are proxied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkerRequest {
    InitInternal {
        handle: u64,
    },
    AddStylesheet {
        handle: u64,
        css: String,
    },
    CreateNode {
        handle: u64,
        node_id: u64,
        text: Option<String>,
    },
    SetParent {
        handle: u64,
        parent_id: u64,
        child_id: u64,
    },
    SetAttribute {
        handle: u64,
        node_id: u64,
        key: String,
        value: String,
    },
    RootId {
        handle: u64,
        reply_to: IpcSender<u64>,
    },
    Run {
        handle: u64,
        reply_to: IpcSender<i32>,
    },
    Destroy {
        handle: u64,
        reply_to: IpcSender<i32>,
    },
    Shutdown,
}
