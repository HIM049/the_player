use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::{fs::File, time::Duration};
use anyhow::{anyhow};
use cpal::traits::{HostTrait, StreamTrait};
use ringbuf::traits::{Observer, Producer};
use ringbuf::{HeapCons, HeapProd, storage::Heap, traits::{Consumer, Split}};
use rodio::{DeviceTrait, decoder};
use symphonia::core::audio::AudioBufferRef;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::ProbeResult;
use symphonia::core::codecs::Decoder;

pub struct MusicCore {
    player: Player,
    decoder: Arc<Mutex<MusicDecoder>>,
    // buffer: SharedRb<Heap<f32>>,
}

impl MusicCore {
    pub fn new() -> Self {
        let capacity = 2048; 
        let rb = ringbuf::SharedRb::<Heap<f32>>::new(capacity);
        let (producer, consumer) = rb.split();

        let player = Player::new(consumer).unwrap();
        let decoder = MusicDecoder::new(producer);

        Self {
            player: player,
            decoder: Arc::new(Mutex::new(decoder)),
            // buffer: todo!(),
        }
    }

    pub fn start_decode(&self) {
        let decoder = self.decoder.clone();
        thread::spawn(move || {
            let mut decoder = decoder.lock().unwrap();
            // let mut sample_rate = codec_params.sample_rate.unwrap_or(44100);
            loop {
                if !decoder.leftover_samples.is_empty() {
                    let mut temp_leftover_samples = std::mem::take(&mut decoder.leftover_samples);
                    let (s1, s2) = temp_leftover_samples.as_slices();
                    let mut written = decoder.producer.push_slice(s1);
                    if written == s1.len() {
                        written += decoder.producer.push_slice(s2);
                    }
                    temp_leftover_samples.drain(..written);
                    decoder.leftover_samples = temp_leftover_samples;
                }

                if decoder.producer.is_full() {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
                
                let package = match decoder.probed.format.next_packet() {
                    Ok(p) => p,
                    Err(_) => break,
                };
                let buff = decoder.decoder.decode(&package).unwrap();

                let (sample, _) = transfer_to_f32(buff);

                let written = decoder.producer.push_slice(&sample);
                if written < sample.len() {
                    let remaining_slice = &sample[written..];
                    decoder.leftover_samples.extend(remaining_slice.iter().cloned());
                }
            }
        });
    }

    pub fn play(&self) {
        self.player.play();
    }
    // pub fn some() {
    //     let host = cpal::default_host();
    //     let device = host.default_output_device()
    //         .expect("no output device available");
    //     let mut supported_configs_range = device.supported_output_configs()
    //         .expect("error while querying configs");
    //     let supported_config = supported_configs_range.next()
    //         .expect("no supported config")
    //         .with_max_sample_rate();

    //     let config = supported_config.config();

    //     let stream = device.build_output_stream(
    //         &config, 
    //         move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
    //             let sample_rate: f32 = 44100.0;
    //             let frequency: f32 = 440.0; 
    //             let phase_increment: f32 = (2.0 * PI * frequency) / sample_rate;
    //             for (index, sample) in data.iter_mut().enumerate() {

    //                 let current_phase = index as f32 * phase_increment;
    //                 let value = current_phase.sin();
                    
    //                 *sample = value * 0.5;
    //             }
    //         },
    //         move |err| {
    //             eprintln!("an error occurred on the output audio stream: {}", err);
    //         },
    //         None
    //     ).unwrap();

    //     stream.play().unwrap();
    //     // stream.pause()
    //     sleep(Duration::from_secs(60));
    // }


}

struct Player {
    host: cpal::Host,
    device: cpal::Device,
    supported_config: cpal::SupportedStreamConfig,
    stream: cpal::Stream,
}

impl Player {
    pub fn new(mut consumer: HeapCons<f32>) -> Result<Self, anyhow::Error> {
        let host = cpal::default_host();
        let device = host.default_output_device()
            .ok_or(anyhow!("no output device available"))?;
        
        let mut supported_configs_range = device.supported_output_configs()?;
        let supported_config = supported_configs_range.next()
            .ok_or(anyhow!("no supported config"))?
            .with_max_sample_rate();

        let stream = device.build_output_stream(
            &supported_config.config(), 
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                consumer.pop_slice(data);
            }, 
            move |err| {}, 
            None
        ).unwrap();

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

pub struct MusicDecoder {
    pub producer: HeapProd<f32>,
    pub probed: Option<ProbeResult>,
    pub decoder: Option<Box<dyn Decoder>>,
    pub leftover_samples: VecDeque<f32>,
}

impl MusicDecoder {
    pub fn new(producer: HeapProd<f32>) -> Self {
        Self {
            producer,
            probed: None,
            decoder: None,
            leftover_samples: VecDeque::new(),
        }
    }

    pub fn from_path(&mut self, file_path: PathBuf) -> Result<Self, anyhow::Error> {
        Ok(self.open_file(Box::new(File::open(file_path)?)))
    }

    pub fn open_file(&mut self, file: Box<File>) -> Self {
        let codecs = symphonia::default::get_codecs();
        let probe = symphonia::default::get_probe();
        let mss = MediaSourceStream::new(file, Default::default());
        let probed = probe.format(
            &Default::default(), 
            mss, 
            &FormatOptions::default(), 
            &MetadataOptions::default()
        ).unwrap();
        // let meta = probed.metadata.get().unwrap();
        let mut format = probed.format;
        // meta.skip_to_latest();
        let track = format.default_track().unwrap();
        let codec_params = track.codec_params.clone();
        let mut decoder = codecs.make(&codec_params, &DecoderOptions::default()).unwrap();


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
        },
        AudioBufferRef::F32(cow) => {
            sample_rate = cow.spec().rate;
            channels = cow.spec().channels.count();
            
            for frame_idx in 0..cow.frames() {
                for ch in 0..channels {
                    let sample = cow.chan(ch)[frame_idx];
                    sample_packet.push(sample);
                }
            }
        },
        AudioBufferRef::F64(cow) => todo!(),
    }
    return (sample_packet, sample_rate)
}