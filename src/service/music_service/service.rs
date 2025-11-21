use std::{
    collections::VecDeque,
    sync::{Arc, atomic::Ordering, mpsc::Sender},
    thread,
    time::Duration,
};

use ringbuf::{
    HeapProd,
    traits::{Observer, Producer},
};
use symphonia::core::formats::Packet;

use crate::service::music_service::{
    controller::Controller, decoder::Decoder, models::Events, stream::Stream, time::PlayTime,
};

pub struct Service {
    // decoder with music
    music_decoder: Decoder,
    // ringbuf peoducer
    producer: HeapProd<f32>,
    // state controller
    controller: Arc<Controller>,

    play_time: Arc<PlayTime>,
    // // device sample rate
    // device_rate: SampleRate,
    // // length of decoded
    // decoded_length: Arc<AtomicU64>,
    // sample write overflow zone
    leftover_samples: VecDeque<f32>,
    // whether need resample
    need_resample: bool,
    // resampler. if need resample, that will be init
    resampler: Option<Stream>,
    // sample pack expected length (for resampler)
    expected_sample_len: usize,

    sender: Option<Sender<Events>>,
}

impl Service {
    pub fn new(
        music_decoder: Decoder,
        producer: HeapProd<f32>,
        controller: Arc<Controller>,
        play_time: Arc<PlayTime>,
    ) -> Self {
        let leftover_samples = VecDeque::new();
        let need_resample = !(play_time.device_sample_rate == music_decoder.sample_rate);
        Self {
            music_decoder,
            producer,
            controller,
            play_time,
            leftover_samples,
            need_resample,
            resampler: None,
            expected_sample_len: 0,
            sender: None,
        }
    }

    pub fn subscribe(mut self, tx: Sender<Events>) -> Self {
        self.sender = Some(tx);
        self
    }

    /// Start decoder thread
    pub fn start_service(mut self) -> Result<(), anyhow::Error> {
        let mut is_finished = false;
        let mut last_playtime: u64 = 0;
        // run decode thread
        thread::spawn(move || {
            loop {
                // if subscribed
                if let Some(tx) = self.sender.as_ref() {
                    // check whether play finished
                    if is_finished {
                        let buf_occupied = self.play_time.occupied_len.load(Ordering::Relaxed);
                        if buf_occupied == 0 {
                            tx.send(Events::PlayFinished).unwrap();
                            break;
                        }
                    }
                    // send current play time
                    let new_playtime = self.play_time.played_sec();
                    if last_playtime < new_playtime {
                        last_playtime = new_playtime;
                        if let Err(e) = tx.send(Events::NewPlaytime(new_playtime)) {
                            eprintln!("error: {}", e);
                        }
                    }
                }

                // stop thread
                if self.controller.is_stopped() {
                    break;
                }

                // pause the thread if need
                self.controller.wait_if_paused();

                // if have overflowed data, push first
                if !self.leftover_samples.is_empty() {
                    let written = self
                        .producer
                        .push_slice(self.leftover_samples.make_contiguous());
                    self.leftover_samples.drain(..written);
                }

                // if ringbuff is full, wait
                if self.producer.is_full() {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }

                // read and decode package data
                let package = match self.music_decoder.format.next_packet() {
                    Ok(p) => p,
                    Err(_) => {
                        is_finished = true;
                        continue;
                    } // play finished
                };

                // decode & transfer & resample
                let (sample, frames) = self.process_stream(&package);

                // append counter
                self.play_time
                    .decoded_len
                    .fetch_add(frames as u64, Ordering::Relaxed);

                // push sample into buffer
                let written = self.producer.push_slice(&sample);
                // buffer is full, put data into overflow
                if written < sample.len() {
                    let remaining_slice = &sample[written..];
                    self.leftover_samples
                        .extend(remaining_slice.iter().cloned());
                }
            }
        });
        Ok(())
    }

    fn process_stream(&mut self, package: &Packet) -> (Vec<f32>, usize) {
        let buff = self.music_decoder.decoder.decode(package).unwrap();
        // transfer data to f32
        let (mut sample, _, channels, frames) = Stream::transfer_to_f32(buff);

        // if need resample
        if self.need_resample {
            // init resampler if not
            if self.resampler.is_none() {
                self.expected_sample_len = sample.len();
                self.resampler = Some(
                    Stream::new(
                        self.music_decoder.sample_rate as usize,
                        self.play_time.device_sample_rate as usize,
                        self.expected_sample_len / channels,
                        channels,
                    )
                    .unwrap(),
                );
            }
            // if short than expected length
            if sample.len() < self.expected_sample_len {
                sample.resize(self.expected_sample_len, 0.0);
            }
            // resample
            sample = self.resampler.as_mut().unwrap().process(&sample);
        }
        (sample, frames)
    }
}
