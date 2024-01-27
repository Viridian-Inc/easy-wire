use std::collections::HashMap;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::Receiver;
use std::thread;
use crate::e_stream::{StreamCore, StreamCoreData, StreamEvent};
use crate::pipe_wire;
use crate::pipe_wire::{PWEvent, NodeEvent, IncomingEvent};


pub struct PipeWireManager<T> {
    pipe_wire: Arc<Mutex<pipe_wire::PipeWire<T>>>,
    // TODO: I don't believe we need to store this information. Let's just implement a get method.
    pub node_info: Arc<Mutex<HashMap<u32, NodeEvent>>>,
    rx: Arc<Mutex<mpsc::Receiver<PWEvent>>>,
    tx: Option<Arc<Mutex<mpsc::Sender<StreamEvent>>>>,
    receiver: Arc<Mutex<Receiver<u32>>>,
}

impl <T>PipeWireManager<T> {
    pub fn new(receive: Receiver<u32>) -> Self {
        let (s_pw_event, r_pwm_process) = mpsc::channel();
        // let (s_pwm_event, r_pw_process) = mpsc::channel();
        let manager = Arc::new(Mutex::new(pipe_wire::PipeWire::new(s_pw_event).unwrap()));
        Self {
            pipe_wire: manager,
            node_info: Arc::new(Mutex::new(HashMap::new())),
            rx: Arc::new(Mutex::new(r_pwm_process)),
            tx: None, // Arc::new(Mutex::new(s_pwm_event)),
            receiver: Arc::new(Mutex::new(receive)),
        }
    }

    pub fn setup_main(&mut self, name: Option<String>, has_listener: bool, stream_core: Option<T>) {
        let pipe_wire = self.pipe_wire.clone();
        let (s_pwm_event, r_pw_process) = mpsc::channel();
        self.tx = Some(Arc::new(Mutex::new(s_pwm_event)));
        if has_listener {
            self.event_loop();
        }
        if stream_core.is_some() {
            self.ev();
        }
        pipe_wire.lock().unwrap().setup_main(name, has_listener,
                                             Some(StreamCoreData {
                                                 receiver: r_pw_process,
                                                 data: stream_core.unwrap(),
                                             })
        );
    }

    fn ev(&mut self) {
        let rx = Arc::clone(&self.receiver);
        let tx = Arc::clone(&self.tx.clone().unwrap());
        thread::spawn(move || {
            for event in rx.lock().unwrap().iter() {
                match event {
                    x => {
                        //println!("Node event: {:?}", x);
                        tx.lock().unwrap().send(StreamEvent::UpdateObjID(x)).unwrap();
                    },
                    _ => {
                        println!("None");
                    },
                }
            }
        });
    }

    fn event_loop(&mut self) {
        let node_info = Arc::clone(&self.node_info);
        let rx = Arc::clone(&self.rx); // Clone the receiver before moving into the thread
        thread::spawn(move || {
            for event in rx.lock().unwrap().iter() {
                match event {
                    PWEvent::Node(node_event) => {
                        println!("Node event: {:?}", node_event);
                        // Find node with object_path with something about google
                        let node = node_info.lock().unwrap().iter().find(|a| a.1.object_path.find("google").is_some());
                        // Now we lock node_info to insert the data
                        node_info.lock().unwrap().insert(node_event.node_id.clone(), node_event);
                    },
                    PWEvent::RemoveNode(node_id) => {
                        node_info.lock().unwrap().remove(&node_id);
                    },
                    // Handle other event variants here
                }
            }
        });
    }
}
