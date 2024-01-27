use std::mem;
use std::sync::{Arc, mpsc, Mutex};
use spa::param::format::{MediaSubtype, MediaType};
use spa::param::format_utils::parse_format;
use spa::pod::Pod;
use crate::core::Core;
use crate::properties::properties;
use crate::{keys};
use crate::pipe_wire::IncomingEvent;
use crate::stream::{Stream, StreamListener, StreamRef};

pub enum StreamEvent {
    UpdateObjID(u32),
}

pub(crate) struct Userdata {
    pub(crate) format: spa::param::audio::AudioInfoRaw,
    pub(crate) cursor_move: bool,
}

pub struct StreamCoreData<T> {
    pub(crate) receiver: mpsc::Receiver<StreamEvent>,
    pub(crate) data: T,
}

pub struct StreamCore<T> {
    pub(crate) core: Arc<Mutex<Option<Core>>>,
    pub(crate) receiver: mpsc::Receiver<StreamEvent>,
    pub(crate) data: T,
}

pub struct EStream<T> {
    pub(crate) core: Core,
    pub(crate) stream: Arc<Mutex<Stream>>,
    pub(crate) receiver: Arc<Mutex<mpsc::Receiver<StreamEvent>>>,
    pub(crate) data: T,
}

fn audio_info() -> crate::spa::pod::Object {
    let mut audio_info = spa::param::audio::AudioInfoRaw::new();
    audio_info.set_format(spa::param::audio::AudioFormat::F32LE);
    crate::spa::pod::Object {
        type_: crate::spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: crate::spa::param::ParamType::EnumFormat.as_raw(),
        properties: audio_info.into(),
    }
}

impl <T>EStream<T> {
    pub fn new(
        stream_core: StreamCore<T>,
    ) -> Self {
        let (stream, core) = Stream::new(&stream_core.core.clone().lock().unwrap().clone().unwrap(), "capture-audio", properties! {
                *keys::NODE_ID => 0.to_string(),
            }).expect("Failed to create stream");
println!("Created stream");
        Self {
            core,
            stream: Arc::new(Mutex::new(stream)),
            receiver: Arc::new(Mutex::new(stream_core.receiver)),
            data: stream_core.data,
        }
    }

    pub fn create_stream(&mut self, node_id: Option<u32>) -> StreamListener<Userdata> {
        println!("Creating stream");
        let receiver = Arc::clone(&self.receiver);
        let mut data = Userdata {
            format: Default::default(),
            cursor_move: false,
        };
        let stream = Arc::clone(&self.stream);

        let _listener = stream.lock().unwrap()
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
                    //user_data.lock().unwrap().into().format.rate(),
                    //user_data.lock().unwrap().into().format.channels()
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
                            StreamEvent::UpdateObjID(obj_id) => {
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
                    //let n_channels = user_data.lock().unwrap().into().format.channels();
                    let n_channels = user_data.format.channels();
                    let n_samples = data.chunk().size() / (mem::size_of::<f32>() as u32);

                    if let Some(samples) = data.data() {
                        //if user_data.lock().unwrap().into().cursor_move {
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
                        //user_data.lock().unwrap().into().cursor_move = true;
                        user_data.cursor_move = true;
                    }
                }
            })
            .register().expect("Failed to create stream");


        let obj = audio_info();
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
        stream.lock().unwrap().connect(
            spa::utils::Direction::Input,
            Some(node_id.unwrap_or(0).clone()),
            crate::stream::StreamFlags::AUTOCONNECT
                | crate::stream::StreamFlags::MAP_BUFFERS
                | crate::stream::StreamFlags::RT_PROCESS,
            &mut params,
        ).expect("Failed to connect stream");
        println!("Connected stream");
        _listener
    }
}

fn change_stream_node(stream: &&StreamRef, obj_id: u32) {
    // let stream = self.stream.clone();
    // let mut old_props = stream.lock().unwrap().unwrap().properties();
    // old_props.insert(*keys::NODE_ID, obj_id.clone().to_string());


    // stream.disconnect().expect("TODO: panic message");
    let mut audio_info = spa::param::audio::AudioInfoRaw::new();
    audio_info.set_format(spa::param::audio::AudioFormat::F32LE);
    let obj = crate::spa::pod::Object {
        type_: crate::spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: obj_id,
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

    stream.update_params(&mut params).expect("Failed to create stream");

    // stream.connect(
    //     spa::utils::Direction::Input,
    //     Some(obj_id),
    //     crate::stream::StreamFlags::AUTOCONNECT
    //         | crate::stream::StreamFlags::MAP_BUFFERS
    //         | crate::stream::StreamFlags::RT_PROCESS,
    //     &mut params,
    // );

}