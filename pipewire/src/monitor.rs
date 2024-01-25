// use std::cell::RefCell;
// use std::collections::HashMap;
// use std::io;
// use std::rc::Rc;
// use nix::sys::signal::Signal;
// use spa::utils::dict::DictRef;
// use crate::keys;
// use crate::port::Port;
// use crate::properties::properties;
// use crate::proxy::{Listener, ProxyListener, ProxyT};
// use crate::types::ObjectType;
//
// struct PipeWireMonitor {
//     proxies: Rc<RefCell<Proxies>>,
//     node_info: HashMap<u32, DictRef>,
// }
//
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
//         }
//     }
//
//     fn add_proxy_t(&mut self, proxy_t: Box<dyn ProxyT>, listener: Box<dyn Listener>) {
//         let proxy_id = {
//             let proxy = proxy_t.upcast_ref();
//             proxy.id()
//         };
//
//         self.proxies_t.insert(proxy_id, proxy_t);
//
//         let v = self.listeners.entry(proxy_id).or_insert_with(Vec::new);
//         v.push(listener);
//     }
//
//     fn add_proxy_listener(&mut self, proxy_id: u32, listener: ProxyListener) {
//         let v = self.listeners.entry(proxy_id).or_insert_with(Vec::new);
//         //println!("add_proxy_listener: {:?}", listener);
//         v.push(Box::new(listener));
//     }
//
//     fn remove(&mut self, proxy_id: u32) {
//         self.proxies_t.remove(&proxy_id);
//         self.listeners.remove(&proxy_id);
//     }
// }
//
//
// impl PipeWireMonitor {
//     fn new() -> Result<Self, String> {
//         let proxies = Rc::new(RefCell::new(Proxies::new()));
//         Ok(Self {
//             proxies,
//             node_info: Default::default(),
//         })
//     }
//
//     fn monitor(&mut self, remote: Option<String>) -> anyhow::Result<()> {
//         let main_loop = crate::main_loop::MainLoop::new()?;
//
//         let main_loop_weak = main_loop.downgrade();
//         let _sig_int = main_loop.add_signal_local(Signal::SIGINT, move || {
//             if let Some(main_loop) = main_loop_weak.upgrade() {
//                 main_loop.quit();
//             }
//         });
//         let main_loop_weak = main_loop.downgrade();
//         let _sig_term = main_loop.add_signal_local(Signal::SIGTERM, move || {
//             if let Some(main_loop) = main_loop_weak.upgrade() {
//                 main_loop.quit();
//             }
//         });
//
//         let context = crate::context::Context::new(&main_loop)?;
//         let props = remote.map(|remote| {
//             properties! {
//             *keys::REMOTE_NAME => remote
//         }
//         });
//         let core = context.connect(props)?;
//
//         // Proxies and their listeners need to stay alive so store them here
//         let proxies = Rc::new(RefCell::new(Proxies::new()));
//
//         let weak_proxies = Rc::downgrade(&proxies);
//
//         let main_loop_weak = main_loop.downgrade();
//         let _listener = core
//             .add_listener_local()
//             .info(|info| {})
//             .done(|_id, _seq| {})
//             .error(move |id, seq, res, message| {
//                 //eprintln!("error id:{} seq:{} res:{}: {}", id, seq, res, message);
//
//                 if id == 0 {
//                     if let Some(main_loop) = main_loop_weak.upgrade() {
//                         main_loop.quit();
//                     }
//                 }
//             })
//             .register();
//
//         let registry = Rc::new(core.get_registry()?);
//         let registry_weak = Rc::downgrade(&registry);
//
//         let _registry_listener = registry
//             .add_listener_local()
//             .global(move |obj| {
//                 if let Some(registry) = registry_weak.upgrade() {
//                     let p: Option<(Box<dyn ProxyT>, Box<dyn Listener>)> = match obj.type_ {
//                         ObjectType::Port => {
//                             let port: Port = registry.bind(obj).unwrap();
//
//                             let obj_listener = port
//                                 .add_listener_local()
//                                 .info(|info| {
//                                     // add info to the stored port
//                                     // info.props().map(|props: DictRef| {
//                                     //     dbg!(props);
//                                     //
//                                     //     props.get(keys::PORT_NAME.clone());
//                                     // });
//                                     //dbg!(info);
//                                 })
//                                 .param(|seq, id, index, next, param| {
//                                     //dbg!((seq, id, index, next, param.map(Pod::as_bytes)));
//                                 })
//                                 .register();
//
//                             Some((Box::new(port), Box::new(obj_listener)))
//                         }
//                         _ => {
//                             //dbg!(obj);
//                             None
//                         }
//                     };
//
//                     if let Some((proxy_spe, listener_spe)) = p {
//                         let proxy = proxy_spe.upcast_ref();
//                         let proxy_id = proxy.id();
//                         // Use a weak ref to prevent references cycle between Proxy and proxies:
//                         // - ref on proxies in the closure, bound to the Proxy lifetime
//                         // - proxies owning a ref on Proxy as well
//                         let proxies_weak = Rc::downgrade(&proxies);
//
//                         let listener = proxy
//                             .add_listener_local()
//                             .removed(move || {
//                                 if let Some(proxies) = proxies_weak.upgrade() {
//                                     proxies.borrow_mut().remove(proxy_id);
//                                 }
//                             })
//                             .register();
//
//                         proxies.borrow_mut().add_proxy_t(proxy_spe, listener_spe);
//                         proxies.borrow_mut().add_proxy_listener(proxy_id, listener);
//                     }
//                     proxies.borrow().get_all_app_audio_nodes();
//                 }
//             })
//             .global_remove(|id| {
//             })
//             .register();
//
//         main_loop.run();
//
//         Ok(())
//     }
//
//     // Additional methods as needed
// }
