use ipc_channel::ipc::IpcSender;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkerEventType {
    Click,
    Scroll,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerEvent {
    pub event_type: WorkerEventType,
    pub x: f64,
    pub y: f64,
    pub scroll_target_id: u64,
    pub scroll_dx: f64,
    pub scroll_dy: f64,
    pub element_ids: Vec<u64>,
}

/// Cross-process requests sent from the host (sonate_lib) to the worker process (sonate_worker).
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
    DestroyNode {
        handle: u64,
        node_id: u64,
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
    SetText {
        handle: u64,
        node_id: u64,
        text: Option<String>,
    },
    RootId {
        handle: u64,
        reply_to: IpcSender<u64>,
    },
    SetEventSender {
        handle: u64,
        sender: IpcSender<WorkerEvent>,
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
