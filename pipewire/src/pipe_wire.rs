use std::collections::HashMap;
use std::{mem, thread};
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Arc, mpsc, Mutex};
use nix::sys::signal::Signal;
use once_cell::race::OnceBox;
use spa::param::format::{MediaSubtype, MediaType};
use spa::param::format_utils::parse_format;
use spa::pod::Pod;
use spa::utils::dict::DictRef;
use crate::{context, keys, main_loop, registry, stream};
use crate::e_stream::{EStream, StreamCore, StreamCoreData};
use crate::port::Port;
use crate::properties::properties;
use crate::proxy::{Listener, ProxyListener, ProxyT};
use crate::stream::{Stream, StreamListener, StreamRef};
use crate::types::ObjectType;


struct Proxies {
    proxies_t: HashMap<u32, Box<dyn ProxyT>>,
    listeners: HashMap<u32, Vec<Box<dyn Listener>>>,
}

impl Proxies {
    fn new() -> Self {
        Self {
            proxies_t: HashMap::new(),
            listeners: HashMap::new(),
        }
    }

    fn add_proxy_t(&mut self, proxy_t: Box<dyn ProxyT>, listener: Box<dyn Listener>) {
        let proxy_id = {
            let proxy = proxy_t.upcast_ref();
            proxy.id()
        };

        self.proxies_t.insert(proxy_id, proxy_t);

        let v = self.listeners.entry(proxy_id).or_insert_with(Vec::new);
        v.push(listener);
    }

    fn add_proxy_listener(&mut self, proxy_id: u32, listener: ProxyListener) {
        let v = self.listeners.entry(proxy_id).or_insert_with(Vec::new);
        //println!("add_proxy_listener: {:?}", listener);
        v.push(Box::new(listener));
    }

    fn remove(&mut self, proxy_id: u32) {
        self.proxies_t.remove(&proxy_id);
        self.listeners.remove(&proxy_id);
    }
}

// TODO: Remove unneeded fields
#[derive(Debug)]
pub struct NodeEvent {
    pub proxy_id: u32,
    pub node_id: u32,
    pub port_id: u32,
    pub channel: String,
    pub object_path: String,
    pub port_name: String,
    pub direction: String,
    pub format: String,
}

pub enum PWEvent {
    Node(NodeEvent),
    RemoveNode(u32),
}

pub enum IncomingEvent {
    UpdateObjID(u32),
}

#[derive(Clone)]
pub struct PipeWire<T> {
    proxies: Arc<Mutex<Proxies>>,
    core: Arc<Mutex<Option<crate::core::Core>>>,
    main_loop: Arc<Mutex<Option<main_loop::MainLoop>>>,
    stream: Arc<Mutex<Option<stream::Stream>>>,
    sender:  Arc<Mutex<mpsc::Sender<PWEvent>>>,
    e_stream: Arc<Mutex<Option<EStream<StreamCore<T>>>>>,
}



impl <T>PipeWire<T> {
    pub fn new(sender:  mpsc::Sender<PWEvent>) -> Result<Self, String> {
        Ok(Self {
            proxies: Arc::new(Mutex::new(Proxies::new())),
            core: Arc::new(Mutex::new(None)),
            main_loop: Arc::new(Mutex::new(None)),
            e_stream: Arc::new(Mutex::new(None)),
            stream: Arc::new(Mutex::new(None)),
            sender: Arc::new(Mutex::new(sender)),
        })
    }

    pub fn setup_main(&mut self,
                      remote: Option<String>,
                      has_listener: bool,
                      mut scd: Option<StreamCoreData<T>>,
    ) {
        let main_loop = Arc::new(Mutex::new(Some(main_loop::MainLoop::new().expect("Failed to create main loop"))));
        let mut main_loop: main_loop::MainLoop = main_loop.clone().lock().unwrap().clone().unwrap();

        //println!("main_loop: {:?}", main_loop);
        let main_loop_weak = main_loop.downgrade();
        let _sig_int = main_loop.add_signal_local(Signal::SIGINT, move || {
            if let Some(main_loop) = main_loop_weak.upgrade() {
                main_loop.quit();
            }
        });
        let main_loop_weak = main_loop.downgrade();
        let _sig_term = main_loop.add_signal_local(Signal::SIGTERM, move || {
            if let Some(main_loop) = main_loop_weak.upgrade() {
                main_loop.quit();
            }
        });

        let context = context::Context::new(&main_loop.clone()).expect("Failed to create context");
        let props = remote.clone().map(|remote| {
            properties! {
            *keys::REMOTE_NAME => remote,
        }
        });
        let core = context.connect(props).expect("Failed to connect to PipeWire");
        self.core = Arc::new(Mutex::new(Some(core)));
        self.main_loop = Arc::new(Mutex::new(Some(main_loop.clone())));
        let proxies_clone = Arc::clone(&self.proxies);
        let mut main_loop: main_loop::MainLoop = self.main_loop.clone().lock().unwrap().clone().unwrap();

        let _listener = match self.core.clone().lock().unwrap().as_ref().unwrap() {
            (ref s) => {
                let main_loop_weak = main_loop.downgrade();
                let reet = s
                    .add_listener_local()
                    .info(|info| {})
                    .done(|_id, _seq| {})
                    .error(move |id, seq, res, message| {
                        if id == 0 {
                            if let Some(main_loop) = main_loop_weak.upgrade() {
                                main_loop.quit();
                            }
                        }
                    })
                    .register();
                reet
            },
        };

        //let _a;
        let _registry_listener;
        let registry = Rc::new(self.core.lock().unwrap().clone().unwrap().get_registry().expect("Failed to get registry"));
        let registry_weak = Rc::downgrade(&registry);
        let tx_lock = Arc::clone(&self.sender);
        let tx_remove = Arc::clone(&self.sender);
        if has_listener {
            //_a = self.setup_listener();
            let proxies_clone = Arc::clone(&self.proxies);




            _registry_listener = registry
                .add_listener_local()
                .global(move |obj| {
                    if let Some(registry) = registry_weak.upgrade() {
                        let p: Option<(Box<dyn ProxyT>, Box<dyn Listener>)> = match obj.type_ {
                            ObjectType::Port => {
                                let port: Port = registry.bind(obj).unwrap();
                                let tx_lock = tx_lock.clone();

                                let obj_listener = port
                                    .add_listener_local()
                                    .info(move |info| {
                                        // add info to the stored port
                                        let props: &DictRef = info.clone().props().unwrap();
                                        match props.get(*keys::NODE_ID) {
                                            None => {},
                                            Some(_) => {
                                                let node_event: NodeEvent = NodeEvent {
                                                    proxy_id: info.id().clone(),
                                                    node_id: props.get(*keys::NODE_ID).unwrap().parse::<u32>().expect("Failed to parse node id"),
                                                    port_id: props.get(*keys::PORT_ID).unwrap().parse::<u32>().expect("Failed to parse port id"),
                                                    channel: props.get(*keys::AUDIO_CHANNEL).unwrap_or("Test").to_string(),
                                                    object_path: props.get(*keys::OBJECT_PATH).unwrap_or("Test").to_string(),
                                                    port_name: props.get(*keys::PORT_NAME).unwrap_or("Test").to_string(),
                                                    direction: props.get(*keys::PORT_DIRECTION).unwrap_or("Test").to_string(),
                                                    format: props.get(*keys::FORMAT_DSP).unwrap_or("Test").to_string(),
                                                };
                                                tx_lock.lock().unwrap().send(PWEvent::Node(node_event)).expect("TODO: panic message");
                                            }
                                        };
                                    })
                                    .param(|seq, id, index, next, param| {})
                                    .register();
                                Some((Box::new(port), Box::new(obj_listener)))
                            }
                            _ => { None }
                        };

                        if let Some((proxy_spe, listener_spe)) = p {
                            let proxy = proxy_spe.upcast_ref();
                            let proxy_id = proxy.id().clone();
                            let tx_remove = tx_remove.clone();
                            let listener = proxy
                                .add_listener_local()
                                .removed(move || {
                                    tx_remove.lock().unwrap().send(PWEvent::RemoveNode(proxy_id)).expect("TODO: panic message");
                                    // TODO: implement this otherwise we will have dead proxies
                                    //proxies_weak.remove(&proxy_id);
                                })
                                .register();

                            proxies_clone.lock().unwrap().add_proxy_t(proxy_spe, listener_spe);
                            proxies_clone.lock().unwrap().add_proxy_listener(proxy_id, listener);
                        }
                    }
                })
                .global_remove(|id| {})
                .register();
            println!("setup_listener: ");
        };


        let _b: EStream<T>;
        let receiver;
        let data;
        let stream_listener;
        if let Some(scd_value) = scd {
            receiver = scd_value.receiver;

            // Prepare the data for the final move
            data = scd_value.data.into(); // `data` is moved here

            _b = EStream::new(StreamCore {
                core: self.core.clone(),
                receiver,
                data, // Use the moved `data` here
            });

            if let Some(mut e_stream) = Some(_b) {//self.e_stream.lock().unwrap().as_mut() {
                stream_listener = e_stream.create_stream(None);
            }
        }
        self.main_loop.lock().unwrap().clone().unwrap().run();
    }
}



pub trait EListener {
    fn setup_listener(&mut self) -> registry::Listener;
}

impl <T>EListener for PipeWire<T> {
    fn setup_listener(&mut self) -> registry::Listener {
        // Proxies and their listeners need to stay alive so store them here
        let proxies_clone = Arc::clone(&self.proxies);


        let registry = Rc::new(self.core.lock().unwrap().clone().unwrap().get_registry().expect("Failed to get registry"));
        let registry_weak = Rc::downgrade(&registry);
        let tx_lock = Arc::clone(&self.sender);
        let tx_remove = Arc::clone(&self.sender);

        let _registry_listener = registry
            .add_listener_local()
            .global(move |obj| {
                if let Some(registry) = registry_weak.upgrade() {
                    let p: Option<(Box<dyn ProxyT>, Box<dyn Listener>)> = match obj.type_ {
                        ObjectType::Port => {
                            let port: Port = registry.bind(obj).unwrap();
                            let tx_lock = tx_lock.clone();

                            let obj_listener = port
                                .add_listener_local()
                                .info(move |info| {
                                    // add info to the stored port
                                    let props: &DictRef = info.clone().props().unwrap();
                                    match props.get(*keys::NODE_ID) {
                                        None => {},
                                        Some(_) => {
                                            let node_event: NodeEvent = NodeEvent {
                                                proxy_id: info.id().clone(),
                                                node_id: props.get(*keys::NODE_ID).unwrap().parse::<u32>().expect("Failed to parse node id"),
                                                port_id: props.get(*keys::PORT_ID).unwrap().parse::<u32>().expect("Failed to parse port id"),
                                                channel: props.get(*keys::AUDIO_CHANNEL).unwrap_or("Test").to_string(),
                                                object_path: props.get(*keys::OBJECT_PATH).unwrap_or("Test").to_string(),
                                                port_name: props.get(*keys::PORT_NAME).unwrap_or("Test").to_string(),
                                                direction: props.get(*keys::PORT_DIRECTION).unwrap_or("Test").to_string(),
                                                format: props.get(*keys::FORMAT_DSP).unwrap_or("Test").to_string(),
                                            };
                                            tx_lock.lock().unwrap().send(PWEvent::Node(node_event)).expect("TODO: panic message");
                                        }
                                    };
                                })
                                .param(|seq, id, index, next, param| {})
                                .register();
                            Some((Box::new(port), Box::new(obj_listener)))
                        }
                        _ => { None }
                    };

                    if let Some((proxy_spe, listener_spe)) = p {
                        let proxy = proxy_spe.upcast_ref();
                        let proxy_id = proxy.id().clone();
                        let tx_remove = tx_remove.clone();
                        let listener = proxy
                            .add_listener_local()
                            .removed(move || {
                                tx_remove.lock().unwrap().send(PWEvent::RemoveNode(proxy_id)).expect("TODO: panic message");
                                // TODO: implement this otherwise we will have dead proxies
                                //proxies_weak.remove(&proxy_id);
                            })
                            .register();

                        proxies_clone.lock().unwrap().add_proxy_t(proxy_spe, listener_spe);
                        proxies_clone.lock().unwrap().add_proxy_listener(proxy_id, listener);
                    }
                }
            })
            .global_remove(|id| {})
            .register();
        println!("setup_listener: ");
        //let obj_listener_boxed: Box<dyn Listener + 'static> = Box::new(_registry_listener) as Box<dyn Listener + 'static>;
        _registry_listener
        //self.main_loop.lock().unwrap().clone().unwrap().run();
    }
}

fn change_stream_node(stream: &&StreamRef, obj_id: u32) {
    // let stream = self.stream.clone();
    // let mut old_props = stream.lock().unwrap().unwrap().properties();
    // old_props.insert(*keys::NODE_ID, obj_id.clone().to_string());


    // stream.disconnect().expect("TODO: panic message");
    // let mut audio_info = spa::param::audio::AudioInfoRaw::new();
    // audio_info.set_format(spa::param::audio::AudioFormat::F32LE);
    // let obj = crate::spa::pod::Object {
    //     type_: crate::spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
    //     id: obj_id,
    //     properties: audio_info.into(),
    // };
    // let values: Vec<u8> = crate::spa::pod::serialize::PodSerializer::serialize(
    //     std::io::Cursor::new(Vec::new()),
    //     &crate::spa::pod::Value::Object(obj),
    // )
    //     .unwrap()
    //     .0
    //     .into_inner();
    //
    // let mut params = [Pod::from_bytes(&values).unwrap()];
    //
    // stream.update_params(&mut params).expect("Failed to create stream");
    //
    // stream.connect(
    //     spa::utils::Direction::Input,
    //     Some(obj_id),
    //     crate::stream::StreamFlags::AUTOCONNECT
    //         | crate::stream::StreamFlags::MAP_BUFFERS
    //         | crate::stream::StreamFlags::RT_PROCESS,
    //     &mut params,
    // );

}


/*pub trait EStream {
    fn setup_stream(&mut self, node_id: Option<u32>) -> StreamListener<UserData>;
    // fn change_stream_node(&mut self, node_id: u32);
}

impl EStream for PipeWire {
    fn setup_stream(&mut self, node_id: Option<u32>) -> StreamListener<UserData>{
        println!("setup_stream: {:?}", node_id);


        let data = UserData {
            format: Default::default(),
            cursor_move: false,
        };
        let receiver = Arc::clone(&self.receiver);

        let _listener = stream
            .add_local_listener_with_user_data(data)
            .param_changed(|_, user_data, id, pod| {
                // NULL means to clear the format
                let Some(pod): Option<&Pod> = pod else {
                    return;
                };
                if id != spa::param::ParamType::Format.as_raw() {
                    return;
                }

                let (media_type, media_subtype) = match parse_format(pod) {
                    Ok(v) => v,
                    Err(_) => return,
                };

                // only accept raw audio
                if media_type != MediaType::Audio || media_subtype != MediaSubtype::Raw {
                    return;
                }

                // call a helper function to parse the format for us.
                user_data
                    .format
                    .parse(pod)
                    .expect("Failed to parse param changed to AudioInfoRaw");

                println!(
                    "capturing rate:{} channels:{}",
                    user_data.format.rate(),
                    user_data.format.channels()
                );
            })
            .process(move |streams, user_data| match streams.dequeue_buffer() {
                None => println!("out of buffers"),
                Some(mut buffer) => {
                    // println!("processing");

                    for event in receiver.lock().unwrap().iter() {
                        match event {
                            IncomingEvent::UpdateObjID(obj_id) => {
                                //println!("UpdateObjID: {:?}", obj_id);
                                change_stream_node(&streams, obj_id);
                            },
                        }
                    }



                    let datas = buffer.datas_mut();
                    if datas.is_empty() {
                        return;
                    }

                    let data = &mut datas[0];
                    let n_channels = user_data.format.channels();
                    let n_samples = data.chunk().size() / (mem::size_of::<f32>() as u32);

                    if let Some(samples) = data.data() {
                        if user_data.cursor_move {
                            print!("\x1B[{}A", n_channels + 1);
                        }
                        match (n_samples, n_channels) {
                            (s, d) if s < 1 || d < 1 => {
                                println!("Failed: samples {} | channels {}", n_samples, n_channels);
                            }
                            (s, d) if s >= 1 && d >= 1 => {
                                println!("Success: captured {} samples", n_samples / n_channels);
                            }
                            _ => (),
                        }

                        for c in 0..n_channels {
                            let mut max: f32 = 0.0;
                            for n in (c..n_samples).step_by(n_channels as usize) {
                                let start = n as usize * mem::size_of::<f32>();
                                let end = start + mem::size_of::<f32>();
                                let chan = &samples[start..end];
                                let f = f32::from_le_bytes(chan.try_into().unwrap());
                                max = max.max(f.abs());
                            }

                            let peak = ((max * 30.0) as usize).clamp(0, 39);

                            println!(
                                "channel {}: |{:>w1$}{:w2$}| peak:{}",
                                c,
                                "*",
                                "",
                                max,
                                w1 = peak + 1,
                                w2 = 40 - peak
                            );
                        }
                        user_data.cursor_move = true;
                    }
                }
            })
            .register().expect("Failed to create stream");

        //self.stream = Arc::new(Mutex::new(Some(stream)));

        let mut audio_info = spa::param::audio::AudioInfoRaw::new();
        audio_info.set_format(spa::param::audio::AudioFormat::F32LE);
        let obj = crate::spa::pod::Object {
            type_: crate::spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
            id: crate::spa::param::ParamType::EnumFormat.as_raw(),
            properties: audio_info.into(),
        };
        let values: Vec<u8> = crate::spa::pod::serialize::PodSerializer::serialize(
            std::io::Cursor::new(Vec::new()),
            &crate::spa::pod::Value::Object(obj),
        )
            .unwrap()
            .0
            .into_inner();

        let mut params = [Pod::from_bytes(&values).unwrap()];

        /* Now connect this stream. We ask that our process function is
         * called in a realtime thread. */
        stream.connect(
            spa::utils::Direction::Input,
            Some(node_id.unwrap_or(0).clone()),
            crate::stream::StreamFlags::AUTOCONNECT
                | crate::stream::StreamFlags::MAP_BUFFERS
                | crate::stream::StreamFlags::RT_PROCESS,
            &mut params,
        ).expect("Failed to connect stream");
        println!("Connected stream");
        _listener
        //self.main_loop.lock().unwrap().clone().unwrap().run();
    }
}*/