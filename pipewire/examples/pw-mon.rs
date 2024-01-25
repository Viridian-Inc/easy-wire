use std::ops::{Deref, DerefMut};
use std::sync::{Arc, mpsc, Mutex};
use std::thread;
use pipewire::{main_loop, manager};
use pipewire::manager::*;


pub struct PipeWireManager {
    pipe_wire: Arc<Mutex<manager::PipeWire>>,
    tx: mpsc::Sender<String>,
    rx: mpsc::Receiver<String>,
}

impl PipeWireManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let manager = Arc::new(Mutex::new(manager::PipeWire::new().unwrap()));
        Self {
            pipe_wire: manager,
            tx,
            rx,
        }
    }

    pub fn setup_main(&mut self, name: Option<&str>) {

    }

    pub fn event_loop(&mut self) {

    }
}

#[tokio::main]
async fn main() {
    let mut manager = Arc::new(Mutex::new(manager::PipeWire::new().unwrap()));
    //let (tx, rx) = mpsc::channel();
    let manager_clone = manager.clone();
    match manager_clone.lock(){
        Ok(mut manager_) => {
            manager_.setup_main(None).await;
            manager_.setup_listener(None);

        },
        Err(_) => panic!("Error: Panick"),
    };
    println!("Program ended")
}