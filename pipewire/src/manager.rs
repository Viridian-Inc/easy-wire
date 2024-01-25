use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use nix::sys::signal::Signal;
use spa::pod::Pod;
use spa::utils::dict::DictRef;
use crate::{context, keys, main_loop};
use crate::main_loop::MainLoop;
use crate::port::Port;
use crate::properties::properties;
use crate::proxy::{Listener, ProxyListener, ProxyT};
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

#[derive(Clone)]
pub struct PipeWire {
    proxies: Arc<Mutex<Proxies>>,
    node_info: Arc<Mutex<HashMap<u32, DictRef>>>,
    core: Arc<Mutex<Option<crate::core::Core>>>,
    main_loop: Arc<Mutex<Option<main_loop::MainLoop>>>,
}

impl PipeWire {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            proxies: Arc::new(Mutex::new(Proxies::new())),
            node_info: Default::default(),
            core: Arc::new(Mutex::new(None)),
            main_loop: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn setup_main(&mut self, remote: Option<String>) {
        let main_loop = Arc::new(Mutex::new(Some(main_loop::MainLoop::new().expect("Failed to create main loop"))));
        let mut main_loop: main_loop::MainLoop = main_loop.clone().lock().unwrap().clone().unwrap();

        println!("main_loop: {:?}", main_loop);
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
        let props = remote.map(|remote| {
            properties! {
            *keys::REMOTE_NAME => remote
        }
        });
        let core = context.connect(props).expect("Failed to connect to PipeWire");
        self.core = Arc::new(Mutex::new(Some(core)));
        self.main_loop = Arc::new(Mutex::new(Some(main_loop.clone())));
    }

    pub fn setup_listener(&mut self, remote: Option<String>) {
        // Proxies and their listeners need to stay alive so store them here
        let proxies = Rc::new(RefCell::new(Proxies::new()));
        let mut main_loop: main_loop::MainLoop = self.main_loop.clone().lock().unwrap().clone().unwrap();

        let _listener = match self.core.clone().lock().unwrap().as_ref().unwrap() {
            (ref s) => {
                let main_loop_weak = main_loop.downgrade();
                let reet = s
                    .add_listener_local()
                    .info(|info| { dbg!(info); })
                    .done(|_id, _seq| {})
                    .error(move |id, seq, res, message| {
                        if id == 0 {
                            if let Some(main_loop) = main_loop_weak.upgrade() {
                                dbg!("error id:{} seq:{} res:{}: {}", id, seq, res, message);
                                main_loop.quit();
                            }
                        }
                    })
                    .register();
                reet
            },
        };

        let registry = Rc::new(self.core.lock().unwrap().clone().unwrap().get_registry().expect("Failed to get registry"));
        let registry_weak = Rc::downgrade(&registry);
        let node_info_lock = Arc::clone(&self.node_info);

        let _registry_listener = registry
            .add_listener_local()
            .global(move |obj| {
                if let Some(registry) = registry_weak.upgrade() {
                    let p: Option<(Box<dyn ProxyT>, Box<dyn Listener>)> = match obj.type_ {
                        ObjectType::Port => {
                            let port: Port = registry.bind(obj).unwrap();
                            let node_info_lock = node_info_lock.clone();

                            let obj_listener = port
                                .add_listener_local()
                                .info(move |info| {
                                    // add info to the stored port
                                    let props: &DictRef = info.clone().props().unwrap();
                                    node_info_lock.lock().unwrap().insert(info.id(), props.clone().into());
                                    // node_info_lock.lock().unwrap().iter().for_each(|(id, info)| {
                                    //     println!("node id: {}", id);
                                    //     println!("node info: {:?}", info);
                                    // });
                                    dbg!(info);
                                    println!("running!");
                                })
                                .param(|seq, id, index, next, param| {
                                    dbg!((seq, id, index, next, param.map(Pod::as_bytes)));
                                })
                                .register();
                            Some((Box::new(port), Box::new(obj_listener)))
                        }
                        _ => {
                            //dbg!(obj);
                            None
                        }
                    };

                    if let Some((proxy_spe, listener_spe)) = p {
                        let proxy = proxy_spe.upcast_ref();
                        let proxy_id = proxy.id();
                        //let proxies_weak = &self.proxies.clone();
                        //let mut proxies_lock = proxies_weak.lock().unwrap();
                        let listener = proxy
                            .add_listener_local()
                            .removed(|| {}
                            //     move || {
                            //     if let (mut proxies) = node_info_lock.lock().unwrap() {
                            //         proxies.remove(&proxy_id);
                            //     }
                            // }
                            )
                            .register();

                        proxies.borrow_mut().add_proxy_t(proxy_spe, listener_spe);
                        proxies.borrow_mut().add_proxy_listener(proxy_id, listener);
                    }
                }
            })
            .global_remove(|id| {})
            .register();
        self.main_loop.lock().unwrap().clone().unwrap().run();
    }

    fn setup_stream(&mut self) {

    }
}
