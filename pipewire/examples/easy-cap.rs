// use anyhow::Result;
// use clap::Parser;
// use pipewire as pw;
// use spa::pod::Pod;
// use std::{cell::RefCell, collections::HashMap, mem, rc::Rc, sync::{Arc, Mutex}};
// use pipewire::{
//     context::Context,
//     main_loop::{MainLoop},
//     registry::GlobalObject,
//     stream::{Stream, StreamFlags},
//     types::ObjectType,
//     node::Node,
//     proxy::{Proxy, ProxyT, Listener},
// };
// use pipewire::properties;
// use pipewire::stream::StreamRef;
// use spa::param::format::MediaType;
// use spa::utils::dict::DictRef;
// use spa::utils::Direction;
//
// struct UserData {
//     format: spa::param::audio::AudioInfoRaw,
//     cursor_move: bool,
// }
//
// // StreamData and CaptureType definitions...
// struct StreamData {
//     node_id: u32,
//     stream_type: MediaType,
//     media_direction: Direction,
//     //props: Option<properties>,
// }
//
// impl StreamData {
//     fn new(node_id: u32, stream_type: MediaType, media_direction: Direction) -> Self {
//         StreamData {
//             node_id,
//             stream_type,
//             media_direction,
//             //props: None,
//         }
//     }
//
//     fn configure_stream(&self, stream: &mut Stream) {
//         match self.stream_type {
//             MediaType::Audio => {
//
//             }
//             MediaType::Video => {
//                 // Configure video stream
//             }
//             MediaType::Unknown => {
//                 println!("Unknown stream type");
//             }
//         }
//     }
//
//     fn handle_stream_events(&self, stream: &StreamRef, user_data: &mut UserData) {
//         match stream {
//             Stream::Audio(stream) => {
//                 match stream.dequeue_buffer() {
//                     None => { println!("out of buffers") },
//                     Some(mut buffer) => {
//                         // Handle audio stream events
//                         let datas = buffer.datas_mut();
//                         if datas.is_empty() {
//                             return;
//                         }
//
//                         let data = &mut datas[0];
//                         let n_channels = user_data.format.channels();
//                         let n_samples = data.chunk().size() / (mem::size_of::<f32>() as u32);
//
//                         if let Some(samples) = data.data() {
//                             if user_data.cursor_move {
//                                 print!("\x1B[{}A", n_channels + 1);
//                             }
//                             match (n_samples, n_channels) {
//                                 (s, d) if s < 1 || d < 1 => {
//                                     println!("Failed: samples {} | channels {}", n_samples, n_channels);
//                                 }
//                                 (s, d) if s >= 1 && d >= 1 => {
//                                     println!("Success: captured {} samples", n_samples / n_channels);
//                                 }
//                                 _ => (),
//                             }
//
//                             for c in 0..n_channels {
//                                 let mut max: f32 = 0.0;
//                                 for n in (c..n_samples).step_by(n_channels as usize) {
//                                     let start = n as usize * mem::size_of::<f32>();
//                                     let end = start + mem::size_of::<f32>();
//                                     let chan = &samples[start..end];
//                                     let f = f32::from_le_bytes(chan.try_into().unwrap());
//                                     max = max.max(f.abs());
//                                 }
//
//                                 let peak = ((max * 30.0) as usize).clamp(0, 39);
//
//                                 println!(
//                                     "channel {}: |{:>w1$}{:w2$}| peak:{}",
//                                     c,
//                                     "*",
//                                     "",
//                                     max,
//                                     w1 = peak + 1,
//                                     w2 = 40 - peak
//                                 );
//                             }
//                             user_data.cursor_move = true;
//                         }
//                     }
//                 }
//             }
//             Stream::Video(stream) => {
//                 println!("We currently do not support video streams");
//             }
//         }
//     }
//
//     fn process_data(&self, data: &[u8]) {
//         // Process the data
//     }
// }
//
// // Proxies struct and implementation...
// struct Proxies {
//     proxies_t: HashMap<u32, Box<dyn ProxyT>>,
//     listeners: HashMap<u32, Vec<Box<dyn Listener>>>,
// }
//
// impl Proxies {
//     fn new() -> Self {
//         Self {
//             proxies_t: HashMap::new(),
//             listeners: HashMap::new(),
//             // Initialize additional fields
//         }
//     }
//
//     fn update_node(&mut self, obj: &GlobalObject<&DictRef>) {
//         // Update nodes logic
//
//     }
//
//     fn remove_node(&mut self, node_id: u32) {
//         // Remove node logic
//     }
//
//     fn get_client_nodes(&self) -> Vec<u32> {
//         // Get client nodes logic
//         Vec::new()
//     }
//
//     fn create_and_setup_stream(&self, node_id: u32, capture_type: MediaType) {
//         // Create and setup stream logic
//     }
//
//     fn start_capturing(&self, node_id: u32, capture_type: MediaType) {
//         // Start capturing logic
//     }
// }
//
// // Main application logic...
// fn monitor(remote: Option<String>) -> Result<()> {
//     let main_loop = MainLoop::new()?;
//     // Signal handling and other setup...
//
//     let context = Context::new(&main_loop)?;
//     let core = context.connect(None)?;
//
//     let proxies = Rc::new(RefCell::new(Proxies::new()));
//
//     // Registry and listener setup...
//
//     main_loop.run();
//
//     Ok(())
// }
//
// #[derive(Parser)]
// #[clap(name = "pw-mon", about = "PipeWire monitor")]
// struct Opt {
//     #[clap(short, long, help = "The name of the remote to connect to")]
//     remote: Option<String>,
// }
//
// fn main() -> Result<()> {
//     pw::init();
//
//     let opt = Opt::parse();
//     monitor(opt.remote)?;
//
//     unsafe {
//         pw::deinit();
//     }
//
//     Ok(())
// }
fn main() {

}