use anyhow::anyhow;
use cpal::traits::{HostTrait, StreamTrait};
use ringbuf::traits::{Observer, Producer};
use ringbuf::{
    HeapCons, HeapProd,
    storage::Heap,
    traits::{Consumer, Split},
};
use rodio::{DeviceTrait, decoder};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::{fs::File, time::Duration};
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::Decoder;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::ProbeResult;

#[derive(PartialEq)]
enum PlayState {
    Playing,
    Paused,
    Stopped,
}
pub struct PlayerControl {
    state: Mutex<PlayState>,
    condvar: Condvar,
}

impl PlayerControl {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(PlayState::Playing),
            condvar: Condvar::new(),
        }
    }
}

// reInit for every new song
pub struct MusicPlayer {
    output: Output,
    controller: Arc<PlayerControl>,
}

impl MusicPlayer {
    pub fn new(file_path: PathBuf) -> Self {
        let capacity = 2048;
        let rb = ringbuf::SharedRb::<Heap<f32>>::new(capacity);
        let (producer, consumer) = rb.split();

        let output = Output::new(consumer).unwrap();
        let controller = Arc::new(PlayerControl::new());
        MusicDecoder::start_decode_from_path(file_path, producer, controller.clone()).unwrap();

        Self { output, controller }
    }

    pub fn play(&self) {
        self.output.play();
    }
}

struct Output {
    host: cpal::Host,
    device: cpal::Device,
    supported_config: cpal::SupportedStreamConfig,
    stream: cpal::Stream,
}

impl Output {
    pub fn new(mut consumer: HeapCons<f32>) -> Result<Self, anyhow::Error> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(anyhow!("no output device available"))?;

        let mut supported_configs_range = device.supported_output_configs()?;
        let supported_config = supported_configs_range
            .next()
            .ok_or(anyhow!("no supported config"))?
            .with_max_sample_rate();

        let stream = device
            .build_output_stream(
                &supported_config.config(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    consumer.pop_slice(data);
                },
                move |err| {},
                None,
            )
            .unwrap();

        Ok(Self {
            host: host,
            device: device,
            supported_config: supported_config,
            stream: stream,
        })
    }

    pub fn play(&self) {
        self.stream.play().unwrap();
    }
}

// TODO: split analysis file and run decoder
// (return a information struct)
pub struct MusicDecoder;
impl MusicDecoder {
    pub fn start_decode_from_path(
        file_path: PathBuf,
        producer: HeapProd<f32>,
        controller: Arc<PlayerControl>,
    ) -> Result<(), anyhow::Error> {
        Self::start_decode(Box::new(File::open(file_path)?), producer, controller)?;
        Ok(())
    }

    pub fn start_decode(
        file: Box<File>,
        mut producer: HeapProd<f32>,
        controller: Arc<PlayerControl>,
    ) -> Result<(), anyhow::Error> {
        let codecs = symphonia::default::get_codecs();
        let probe = symphonia::default::get_probe();
        let mss = MediaSourceStream::new(file, Default::default());
        let probed = probe
            .format(
                &Default::default(),
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .unwrap();
        // let meta = probed.metadata.get().unwrap();
        let mut format = probed.format;
        // meta.skip_to_latest();
        let track = format.default_track().unwrap();
        let codec_params = track.codec_params.clone();
        let mut decoder = codecs
            .make(&codec_params, &DecoderOptions::default())
            .unwrap();

        let mut leftover_samples = VecDeque::new();
        thread::spawn(move || {
            // let mut sample_rate = codec_params.sample_rate.unwrap_or(44100);
            loop {
                let mut state_guard = controller.state.lock().unwrap();
                // pause the thread
                while *state_guard == PlayState::Paused {
                    state_guard = controller.condvar.wait(state_guard).unwrap();
                }

                if *state_guard == PlayState::Stopped {
                    break;
                }

                if !leftover_samples.is_empty() {
                    let written = producer.push_slice(leftover_samples.make_contiguous());
                    leftover_samples.drain(..written);
                }

                if producer.is_full() {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }

                let package = match format.next_packet() {
                    Ok(p) => p,
                    Err(_) => break, // play finished
                };
                let buff = decoder.decode(&package).unwrap();

                let (sample, _) = transfer_to_f32(buff);

                let written = producer.push_slice(&sample);
                if written < sample.len() {
                    let remaining_slice = &sample[written..];
                    leftover_samples.extend(remaining_slice.iter().cloned());
                }
            }
        });
        Ok(())
    }
}

fn transfer_to_f32(buff: AudioBufferRef) -> (Vec<f32>, u32) {
    let mut sample_packet = vec![];
    let sample_rate: u32;
    let mut channels: usize = 1;
    match buff {
        AudioBufferRef::U8(cow) => todo!(),
        AudioBufferRef::U16(cow) => todo!(),
        AudioBufferRef::U24(cow) => todo!(),
        AudioBufferRef::U32(cow) => todo!(),
        AudioBufferRef::S8(cow) => todo!(),
        AudioBufferRef::S16(cow) => todo!(),
        AudioBufferRef::S24(cow) => todo!(),
        AudioBufferRef::S32(cow) => {
            sample_rate = cow.spec().rate;
            for frame_idx in 0..cow.frames() {
                for ch in 0..channels {
                    let sample = cow.chan(ch)[frame_idx] as f32 / i32::MAX as f32;
                    sample_packet.push(sample)
                }
            }
        }
        AudioBufferRef::F32(cow) => {
            sample_rate = cow.spec().rate;
            channels = cow.spec().channels.count();

            for frame_idx in 0..cow.frames() {
                for ch in 0..channels {
                    let sample = cow.chan(ch)[frame_idx];
                    sample_packet.push(sample);
                }
            }
        }
        AudioBufferRef::F64(cow) => {
            sample_rate = cow.spec().rate;
            channels = cow.spec().channels.count();
            for frame_idx in 0..cow.frames() {
                for ch in 0..channels {
                    let sample = cow.chan(ch)[frame_idx] as f32;
                    sample_packet.push(sample);
                }
            }
        }
    }
    return (sample_packet, sample_rate);
}
