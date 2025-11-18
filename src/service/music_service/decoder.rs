use anyhow::anyhow;
use cpal::SampleRate;
use ringbuf::HeapProd;
use ringbuf::traits::{Observer, Producer};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::thread;
use std::{fs::File, time::Duration};
use symphonia::core::codecs::{self, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use crate::service::music_service::controller::Controller;
use crate::service::music_service::models;
use crate::service::music_service::stream::Stream;

pub struct Decoder {
    pub sample_rate: u32,
    pub format: Box<dyn FormatReader>,
    pub decoder: Box<dyn codecs::Decoder>,
}

impl Decoder {
    /// Decode from a file path
    pub fn decode_from_path(file_path: PathBuf) -> Result<Self, anyhow::Error> {
        Ok(Self::decode_file(Box::new(File::open(file_path)?))?)
    }

    /// Decode from file
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
        let track = format
            .default_track()
            .ok_or_else(|| anyhow!("no track found"))?;
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

    /// Start decoder thread
    pub fn start_decoder(
        mut music_decoder: Decoder,
        mut producer: HeapProd<f32>,
        controller: Arc<Controller>,
        device_rate: SampleRate,
        decoded_len: Arc<AtomicU64>,
    ) -> Result<(), anyhow::Error> {
        // sample write overflow zone
        let mut leftover_samples = VecDeque::new();
        // whether need resample
        let need_resample = !(device_rate.0 == music_decoder.sample_rate);
        // resampler. if need resample, that will be init
        let mut resampler: Option<Stream> = None;
        // sample pack expected length
        let mut expected_sample_len: usize = 0;
        // let mut final_len: Option<usize> = None;

        // run decode thread
        thread::spawn(move || {
            loop {
                let rate = producer.occupied_len() as f32 / models::RINGBUF_SIZE as f32;
                // pause the thread if need
                controller.wait_if_paused();

                // stop thread
                if controller.is_stopped() {
                    break;
                }

                // if have overflowed data, push first
                if !leftover_samples.is_empty() {
                    let written = producer.push_slice(leftover_samples.make_contiguous());
                    leftover_samples.drain(..written);
                }

                // if ringbuff is full, wait
                if producer.is_full() {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }

                // read and decode package data
                let package = match music_decoder.format.next_packet() {
                    Ok(p) => p,
                    Err(_) => break, // play finished
                };
                let buff = music_decoder.decoder.decode(&package).unwrap();
                // transfer data to f32
                let (mut sample, _, channels, frames) = Stream::transfer_to_f32(buff);
                // append counter
                decoded_len.fetch_add(frames as u64, std::sync::atomic::Ordering::Relaxed);

                // if need resample
                if need_resample {
                    // init resampler if not
                    if resampler.is_none() {
                        expected_sample_len = sample.len();
                        resampler = Some(
                            Stream::new(
                                music_decoder.sample_rate as usize,
                                device_rate.0 as usize,
                                expected_sample_len / channels,
                                channels,
                            )
                            .unwrap(),
                        );
                    }
                    // if short than expected length
                    if sample.len() < expected_sample_len {
                        sample.resize(expected_sample_len, 0.0);
                    }
                    // resample
                    sample = resampler.as_mut().unwrap().process(&sample);
                };

                // push sample into buffer
                let written = producer.push_slice(&sample);
                // buffer is full, put data into overflow
                if written < sample.len() {
                    let remaining_slice = &sample[written..];
                    leftover_samples.extend(remaining_slice.iter().cloned());
                }
            }
        });
        Ok(())
    }
}
