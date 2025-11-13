use anyhow::anyhow;
use cpal::SampleRate;
use cpal::traits::{HostTrait, StreamTrait};
use ringbuf::traits::{Observer, Producer};
use ringbuf::{
    HeapCons, HeapProd,
    storage::Heap,
    traits::{Consumer, Split},
};
use rodio::DeviceTrait;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::{fs::File, time::Duration};
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

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

        let decoded = MusicDecoder::decode_from_path(file_path).unwrap();
        let output = Output::new(consumer, SampleRate(decoded.sample_rate)).unwrap();
        let controller = Arc::new(PlayerControl::new());
        MusicDecoder::start_decoder(decoded, producer, controller.clone()).unwrap();

        Self { output, controller }
    }

    pub fn play(&self) {
        let mut state = self.controller.state.lock().unwrap();
        *state = PlayState::Playing;
        self.controller.condvar.notify_one();
        self.output.play();
    }

    pub fn pause(&self) {
        self.output.pause();
        let mut state = self.controller.state.lock().unwrap();
        *state = PlayState::Paused;
    }
}

struct Output {
    host: cpal::Host,
    device: cpal::Device,
    supported_config: cpal::StreamConfig,
    stream: cpal::Stream,
}

impl Output {
    pub fn new(
        mut consumer: HeapCons<f32>,
        target_sample_rate: SampleRate,
    ) -> Result<Self, anyhow::Error> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(anyhow!("no output device available"))?;

        // init config
        let mut supported_configs_range = device.supported_output_configs()?;
        let support_config_range = supported_configs_range.find(|config| {
            config.min_sample_rate() <= target_sample_rate
                && target_sample_rate <= config.max_sample_rate()
        });
        let supported_config;
        if let Some(config) = support_config_range {
            supported_config = config.with_sample_rate(target_sample_rate).config();
        } else {
            supported_config = supported_configs_range
                .next()
                .ok_or(anyhow!("no supported config"))?
                .with_max_sample_rate()
                .config();
        }

        let stream = device
            .build_output_stream(
                &supported_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    consumer.pop_slice(data);
                },
                move |err| {
                    eprintln!("error: {}", err);
                },
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

    pub fn pause(&self) {
        self.stream.pause().unwrap();
    }
}

pub struct MusicDecoder {
    pub sample_rate: u32,
    pub format: Box<dyn FormatReader>,
    pub decoder: Box<dyn Decoder>,
}
impl MusicDecoder {
    pub fn decode_from_path(file_path: PathBuf) -> Result<Self, anyhow::Error> {
        Ok(Self::decode_file(Box::new(File::open(file_path)?))?)
    }

    pub fn decode_file(file: Box<File>) -> Result<Self, anyhow::Error> {
        let probe = symphonia::default::get_probe();
        let mss = MediaSourceStream::new(file, Default::default());
        let probed = probe.format(
            &Default::default(),
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?;

        let format = probed.format;
        let track = format.default_track().unwrap();
        let codec_params = track.codec_params.clone();
        let sample_rate = codec_params.sample_rate.unwrap_or(44100);

        let codecs = symphonia::default::get_codecs();
        let decoder = codecs.make(&codec_params, &DecoderOptions::default())?;
        Ok(Self {
            sample_rate,
            format,
            decoder,
        })
    }

    pub fn start_decoder(
        mut music_decoder: MusicDecoder,
        mut producer: HeapProd<f32>,
        controller: Arc<PlayerControl>,
    ) -> Result<(), anyhow::Error> {
        let mut leftover_samples = VecDeque::new();
        thread::spawn(move || {
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

                let package = match music_decoder.format.next_packet() {
                    Ok(p) => p,
                    Err(_) => break, // play finished
                };
                let buff = music_decoder.decoder.decode(&package).unwrap();

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
        AudioBufferRef::U8(cow) => {
            sample_rate = cow.spec().rate;
            channels = cow.spec().channels.count();
            for frame_idx in 0..cow.frames() {
                for ch in 0..channels {
                    let raw = cow.chan(ch)[frame_idx];
                    let norm = raw as f32 / u8::MAX as f32;
                    let sample = norm * 2.0 - 1.0;
                    sample_packet.push(sample)
                }
            }
        }
        AudioBufferRef::U16(cow) => {
            sample_rate = cow.spec().rate;
            channels = cow.spec().channels.count();
            for frame_idx in 0..cow.frames() {
                for ch in 0..channels {
                    let raw = cow.chan(ch)[frame_idx];
                    let norm = raw as f32 / u16::MAX as f32;
                    let sample = norm * 2.0 - 1.0;
                    sample_packet.push(sample)
                }
            }
        }
        AudioBufferRef::U24(cow) => {
            sample_rate = cow.spec().rate;
            channels = cow.spec().channels.count();
            for frame_idx in 0..cow.frames() {
                for ch in 0..channels {
                    let raw = cow.chan(ch)[frame_idx].0;
                    let norm = raw as f32 / 16_777_215.0;
                    let sample = norm * 2.0 - 1.0;
                    sample_packet.push(sample)
                }
            }
        }
        AudioBufferRef::U32(cow) => {
            sample_rate = cow.spec().rate;
            channels = cow.spec().channels.count();
            for frame_idx in 0..cow.frames() {
                for ch in 0..channels {
                    let raw = cow.chan(ch)[frame_idx];
                    let norm = raw as f32 / u32::MAX as f32;
                    let sample = norm * 2.0 - 1.0;
                    sample_packet.push(sample)
                }
            }
        }
        AudioBufferRef::S8(cow) => {
            sample_rate = cow.spec().rate;
            channels = cow.spec().channels.count();
            for frame_idx in 0..cow.frames() {
                for ch in 0..channels {
                    let sample = cow.chan(ch)[frame_idx] as f32 / i8::MAX as f32;
                    sample_packet.push(sample)
                }
            }
        }
        AudioBufferRef::S16(cow) => {
            sample_rate = cow.spec().rate;
            channels = cow.spec().channels.count();
            for frame_idx in 0..cow.frames() {
                for ch in 0..channels {
                    let sample = cow.chan(ch)[frame_idx] as f32 / i16::MAX as f32;
                    sample_packet.push(sample)
                }
            }
        }
        AudioBufferRef::S24(cow) => {
            sample_rate = cow.spec().rate;
            channels = cow.spec().channels.count();
            for frame_idx in 0..cow.frames() {
                for ch in 0..channels {
                    let raw_val = cow.chan(ch)[frame_idx].0;
                    let sample = raw_val as f32 / 8_388_608.0; // 2^23
                    sample_packet.push(sample)
                }
            }
        }
        AudioBufferRef::S32(cow) => {
            sample_rate = cow.spec().rate;
            channels = cow.spec().channels.count();
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
