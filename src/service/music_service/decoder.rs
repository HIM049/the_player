use cpal::SampleRate;
use ringbuf::HeapProd;
use ringbuf::traits::{Observer, Producer};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::{fs::File, time::Duration};
use symphonia::core::codecs::{self, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use crate::service::music_service::controller::Controller;
use crate::service::music_service::stream::Stream;

pub struct Decoder {
    pub sample_rate: u32,
    pub format: Box<dyn FormatReader>,
    pub decoder: Box<dyn codecs::Decoder>,
}
impl Decoder {
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
        mut music_decoder: Decoder,
        mut producer: HeapProd<f32>,
        controller: Arc<Controller>,
        device_rate: SampleRate,
    ) -> Result<(), anyhow::Error> {
        let mut leftover_samples = VecDeque::new();
        let need_resample = !(device_rate.0 == music_decoder.sample_rate);
        let mut resampler: Option<Stream> = None;
        let mut expected_sample_len: usize = 0;

        thread::spawn(move || {
            loop {
                // pause the thread if need
                controller.wait_if_paused();

                // stop thread
                if controller.is_stopped() {
                    break;
                }

                // if have overflow, push first
                if !leftover_samples.is_empty() {
                    let written = producer.push_slice(leftover_samples.make_contiguous());
                    leftover_samples.drain(..written);
                }

                // if ringbuff is full, wait
                if producer.is_full() {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }

                let package = match music_decoder.format.next_packet() {
                    Ok(p) => p,
                    Err(_) => break, // play finished
                };
                let buff = music_decoder.decoder.decode(&package).unwrap();
                let (mut sample, _, channels) = Stream::transfer_to_f32(buff);

                // if need resample
                if need_resample {
                    if resampler.is_none() {
                        resampler = Some(
                            Stream::new(
                                music_decoder.sample_rate as usize,
                                device_rate.0 as usize,
                                sample.len() / channels,
                                channels,
                            )
                            .unwrap(),
                        );
                        expected_sample_len = sample.len();
                    }
                    // if short than expected length
                    if sample.len() < expected_sample_len {
                        sample.resize(expected_sample_len, 0.0);
                    }
                    sample = resampler.as_mut().unwrap().process(&sample);
                };

                // push sample into buffer
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
