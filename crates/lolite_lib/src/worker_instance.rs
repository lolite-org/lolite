use ipc_channel::ipc::{IpcReceiver, IpcSender};
use shared_memory::{ShmemConf, ShmemError};
use std::{
    path::Path,
    process::{Command, Stdio},
    sync::mpsc::{channel, Receiver, Sender},
};

const WORKER_FILE: &str = "lolite_worker.exe";

pub struct WorkerInstance {
    #[allow(dead_code)]
    process: std::process::Child,
    sender: Sender<MemoryMessage>,
}

impl WorkerInstance {
    pub fn new() -> std::io::Result<WorkerInstance> {
        Self::new_using_ipc_channel()
    }

    // initial idea, not functional yet
    pub fn new_using_shmem() -> std::io::Result<WorkerInstance> {
        // Set up shared memory
        let (sender, thread_receiver) = channel();
        let (thread_sender, receiver) = channel();

        std::thread::spawn(move || shmem_thread(thread_receiver, thread_sender));

        let shmem_osid = match receiver.recv() {
            // errors
            Ok(MemoryResponse::ShmemError(e)) => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Shmem error: {e}"),
            )),
            Err(e) => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error receiving response: {e}"),
            )),
            // pog
            Ok(MemoryResponse::Osid(osid)) => Ok(osid),
            Ok(MemoryResponse::Ipc(_, _)) => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unexpected IPC response in shmem method".to_string(),
            )),
        }?;

        println!("Shmem osid: {shmem_osid}");

        let process = spawn_worker("shmem", &shmem_osid)?;

        // Pog
        Ok(WorkerInstance { process, sender })
    }

    // second idea, maybe simpler to implement? No idea how performance will be
    pub fn new_using_ipc_channel() -> std::io::Result<WorkerInstance> {
        // Set up shared memory
        let (sender, thread_receiver) = channel();
        let (thread_sender, receiver) = channel();

        std::thread::spawn(move || memory_thread(thread_receiver, thread_sender));

        let shmem_osid = match receiver.recv() {
            // errors
            Ok(MemoryResponse::ShmemError(e)) => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Shmem error: {e}"),
            )),
            Err(e) => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error receiving response: {e}"),
            )),
            // pog
            Ok(MemoryResponse::Osid(osid)) => Ok(osid),
            Ok(MemoryResponse::Ipc(_, _)) => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unexpected IPC response when expecting OSID".to_string(),
            )),
        }?;

        println!("Shmem osid: {shmem_osid}");

        // Make process
        let process = spawn_worker(&"ipc_channel", &shmem_osid)?;

        // Pog
        Ok(WorkerInstance { process, sender })
    }

    pub fn add_element(&mut self, element: &lolite_transfer::Element) {
        let mut data = vec![];
        data.reserve(1024);

        lolite_transfer::encode_element(&mut data, element);

        self.sender.send(MemoryMessage::Send(data)).unwrap();
    }
}

enum MemoryMessage {
    Send(Vec<u8>),
}

enum MemoryResponse {
    ShmemError(ShmemError),
    Osid(String),
    Ipc(IpcSender<Vec<u8>>, IpcReceiver<Vec<u8>>),
}

fn shmem_thread(request: Receiver<MemoryMessage>, response: Sender<MemoryResponse>) {
    // make shared memory
    let mut shmem = match ShmemConf::new().size(1024 * 1024).create() {
        Ok(m) => m,
        Err(e) => {
            response.send(MemoryResponse::ShmemError(e)).unwrap();
            return;
        }
    };

    response
        .send(MemoryResponse::Osid(shmem.get_os_id().to_owned()))
        .unwrap();

    // make ipc channel for communication
    let (my_sender, their_receiver) = ipc_channel::ipc::channel().unwrap();
    let (their_sender, _my_receiver) = ipc_channel::ipc::channel().unwrap();

    response
        .send(MemoryResponse::Ipc(their_sender, their_receiver))
        .unwrap();

    // event loop
    loop {
        match request.recv() {
            Ok(MemoryMessage::Send(new_data)) => unsafe {
                let slice = shmem.as_slice_mut();

                if new_data.len() > slice.len() {
                    eprintln!("Data too large for shared memory");
                    continue;
                }

                println!("Sending message {:?}", &new_data);

                slice[..new_data.len()].copy_from_slice(&new_data);

                my_sender.send(new_data).unwrap();
            },
            Err(e) => {
                eprintln!("Error receiving message: {e}");
                return;
            }
        }
    }
}

fn memory_thread(request: Receiver<MemoryMessage>, response: Sender<MemoryResponse>) {
    // make shared memory
    // let mut shmem = match ShmemConf::new().size(1024 * 1024).create() {
    //     Ok(m) => m,
    //     Err(e) => {
    //         response.send(MemoryResponse::ShmemError(e)).unwrap();
    //         return;
    //     }
    // };

    // response
    //     .send(MemoryResponse::Osid(shmem.get_os_id().to_owned()))
    //     .unwrap();

    // make ipc channel
    let (my_sender, their_receiver) = ipc_channel::ipc::channel().unwrap();
    let (their_sender, my_receiver) = ipc_channel::ipc::channel().unwrap();

    response
        .send(MemoryResponse::Ipc(their_sender, their_receiver))
        .unwrap();

    // event loop
    loop {
        match request.recv() {
            Ok(MemoryMessage::Send(new_data)) => unsafe {
                //let slice = shmem.as_slice_mut();

                //if new_data.len() > slice.len() {
                //    eprintln!("Data too large for shared memory");
                //    continue;
                //}

                //println!("Sending message {:?}", &new_data);

                //slice[..new_data.len()].copy_from_slice(&new_data);

                my_sender.send(new_data).unwrap();
            },
            Err(e) => {
                eprintln!("Error receiving message: {e}");
                return;
            }
        }
    }
}

fn spawn_worker(method: &str, connection_key: &str) -> std::io::Result<std::process::Child> {
    let exe_dir = std::env::current_exe();
    let worker_dir = match &exe_dir {
        Ok(path) => path.parent().unwrap(),
        Err(_) => Path::new("."),
    };
    let worker_path = worker_dir.join(WORKER_FILE);
    println!("Running worker at {worker_path:?}");

    Command::new(worker_path)
        .arg(method)
        .arg(connection_key)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
}
