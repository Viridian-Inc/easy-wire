// Copyright The pipewire-rs Contributors.
// SPDX-License-Identifier: MIT

//! This file is a rustic interpretation of the [PipeWire audio-capture.c example][example]
//!
//! example: https://docs.pipewire.org/audio-capture_8c-example.html

use clap::Parser;
use pipewire as pw;
use pw::{spa};
use spa::param::format_utils::{parse_format};
use spa::pod::Pod;
use std::convert::TryInto;
use std::mem;
use pipewire::properties::properties;
use spa::param::format::{MediaSubtype, MediaType};

struct UserData {
    format: spa::param::audio::AudioInfoRaw,
    cursor_move: bool,
}

#[derive(Parser)]
#[clap(name = "audio-capture", about = "Audio stream capture example")]
struct Opt {
    #[clap(short, long, help = "The target object id to connect to")]
    target: Option<String>,
}

struct Streams {
    streams: Vec<pw::stream::Stream>,
}

impl Streams {
    fn new() -> Self {
        Self {
            streams: Vec::new(),
        }
    }

    fn add_stream(&mut self, stream: pw::stream::Stream) {
        self.streams.push(stream);
    }

    fn remove_stream(&mut self, stream: pw::stream::Stream) {
        self.streams.retain(|s| s.name() != stream.name());
    }
}

pub fn main() -> Result<(), pw::Error> {
    pw::init();

    let mainloop = pw::main_loop::MainLoop::new()?;
    let context = pw::context::Context::new(&mainloop)?;
    let core = context.connect(None)?;

    let data = UserData {
        format: Default::default(),
        cursor_move: false,
    };

    /* Create a simple stream, the simple stream manages the core and remote
     * objects for you if you don't need to deal with them.
     *
     * If you plan to autoconnect your stream, you need to provide at least
     * media, category and role properties.
     *
     * Pass your events and a user_data pointer as the last arguments. This
     * will inform you about the stream state. The most important event
     * you need to listen to is the process event where you need to produce
     * the data.
     */
    #[cfg(not(feature = "v0_3_44"))]
        let mut props = properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Capture",
        *pw::keys::MEDIA_ROLE => "Music",
    };
    #[cfg(feature = "v0_3_44")]
        let mut props = {
        let opt = Opt::parse();

        let mut props = properties! {
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Music",
        };
        if let Some(target) = opt.target {
            props.insert(*pw::keys::TARGET_OBJECT, target);
        }
        props
    };

    // uncomment if you want to capture from the sink monitor ports
    props.insert(*pw::keys::STREAM_CAPTURE_SINK, "true");

    // Capture app audio via NODE_ID
    //props.insert(*pw::keys::NODE_ID, "145"); // Set to the node id of the application you would like to capture
    // Capture app audio via OBJECT_ID
    //props.insert(*pw::keys::OBJECT_ID, "145"); // Set to the object id of the application you would like to capture
    // Capture app audio via OBJECT_SERIAL
    //props.insert(*pw::keys::OBJECT_SERIAL, "145"); // Set to the object serial of the application you would like to capture

    let (stream, _core) = pw::stream::Stream::new(&core, "capture-audio", props)?;

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
        .process(|stream, user_data| match stream.dequeue_buffer() {
            None => println!("out of buffers"),
            Some(mut buffer) => {
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
        .register()?;

    /* Make one parameter with the supported formats. The SPA_PARAM_EnumFormat
     * id means that this is a format enumeration (of 1 value).
     * We leave the channels and rate empty to accept the native graph
     * rate and channels. */
    let mut audio_info = spa::param::audio::AudioInfoRaw::new();
    audio_info.set_format(spa::param::audio::AudioFormat::F32LE);
    let obj = pw::spa::pod::Object {
        type_: pw::spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: pw::spa::param::ParamType::EnumFormat.as_raw(),
        properties: audio_info.into(),
    };
    let values: Vec<u8> = pw::spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pw::spa::pod::Value::Object(obj),
    )
    .unwrap()
    .0
    .into_inner();

    let mut params = [Pod::from_bytes(&values).unwrap()];

    /* Now connect this stream. We ask that our process function is
     * called in a realtime thread. */
    stream.connect(
        spa::utils::Direction::Input,
        Some(5254),
        pw::stream::StreamFlags::AUTOCONNECT
            | pw::stream::StreamFlags::MAP_BUFFERS
            | pw::stream::StreamFlags::RT_PROCESS,
        &mut params,
    )?;

    // and wait while we let things run
    mainloop.run();

    Ok(())
}
