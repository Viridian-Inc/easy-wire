use std::ops::{Deref, DerefMut};
use std::sync::{Arc, mpsc, Mutex};
use std::thread;
use pipewire::{main_loop, pipe_wire};
use pipewire::pipe_wire::*;
use pipewire::pipe_wire_manager::PipeWireManager;


#[tokio::main]
async fn main() {
    let mut manager = PipeWireManager::new();
    manager.setup_main(None).await;
    println!("Program ended")
}