use std::sync::{Arc, mpsc, Mutex};
use std::thread;
use tokio::task;
use crate::pipe_wire;

pub struct PipeWireManager {
    pipe_wire: Arc<Mutex<pipe_wire::PipeWire>>,
    tx: mpsc::Sender<String>,
    rx: mpsc::Receiver<String>,
}

impl PipeWireManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let manager = Arc::new(Mutex::new(pipe_wire::PipeWire::new().unwrap()));
        Self {
            pipe_wire: manager,
            tx,
            rx,
        }
    }

    pub async fn setup_main(&mut self, name: Option<&str>) {
        let pipe_wire = self.pipe_wire.clone();
        task::spawn(async move {
            match pipe_wire.lock() {
                Ok(mut manager) => {
                    manager.setup_main(None).await;
                    manager.setup_listener(None);
                },
                Err(_) => panic!("Error: Panick"),
            };
        });
    }

    pub fn event_loop(&mut self) {

    }
}

// unsafe impl Send for PipeWireManager {
//     fn send(&self, message: String) -> Result<(), String> {
//         self.tx.send(message).unwrap();
//         Ok(())
//     }
// }