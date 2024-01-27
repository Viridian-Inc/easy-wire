use std::ops::{Deref, DerefMut};
use std::sync::{Arc, mpsc, Mutex};
use std::thread;
use std::thread::Thread;
use libc::sleep;
use pipewire::{main_loop, pipe_wire};
use pipewire::pipe_wire::*;
use pipewire::pipe_wire_manager::PipeWireManager;

struct Userdata {
    pub(crate) format: spa::param::audio::AudioInfoRaw,
    pub(crate) cursor_move: bool,
}

fn main() {
    let (send, receive) = mpsc::channel();
    thread::spawn(move || {
        let mut manager = PipeWireManager::new(receive);
        let user_data = Userdata {
            format: spa::param::audio::AudioInfoRaw::new(),
            cursor_move: false,
        };

        manager.setup_main(None, true, Some(user_data));
    });

    //thread::spawn(move || {
    //unsafe { sleep(3); }
    send.send(72).unwrap();
        loop {

            //println!("Sent message");
        }
    //});

    // find a node that has google in the object_path it may not be the full string
    //println!("Program ended")
}
