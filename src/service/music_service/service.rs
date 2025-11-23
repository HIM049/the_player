use std::{
    collections::VecDeque,
    sync::{Arc, atomic::Ordering},
    thread,
    time::Duration,
};

use ringbuf::{
    HeapProd,
    traits::{Observer, Producer},
};
use smol::channel::Sender;
use symphonia::core::formats::{Packet, SeekMode, SeekTo};

use crate::service::music_service::{
    controller::{Controller, ServiceState},
    decoder::Decoder,
    models::Events,
    stream::Stream,
    time::PlayTime,
};

pub struct Service {
    // decoder with music
    music_decoder: Decoder,
    // ringbuf peoducer
    producer: HeapProd<f32>,
    // state controller
    controller: Arc<Controller>,
    // play time
    play_time: Arc<PlayTime>,
    // sample write overflow zone
    leftover_samples: VecDeque<f32>,
    // whether need resample
    need_resample: bool,
    // resampler. if need resample, that will be init
    resampler: Option<Stream>,
    // sample pack expected length (for resampler)
    expected_sample_len: usize,
    // channel sender
    sender: Option<Sender<Events>>,
}

impl Service {
    // Create new service
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

    /// subscribe play events
    pub fn subscribe(mut self, tx: Sender<Events>) -> Self {
        self.sender = Some(tx);
        self
    }

    /// Start decoder thread
    pub fn start_service(mut self) -> Result<(), anyhow::Error> {
        let mut is_finished = false;
        let mut last_sent_time: f64 = 0.0;
        // run decode thread
        thread::spawn(move || {
            loop {
                // state check
                match self.controller.state() {
                    ServiceState::Playing => {
                        // if subscribed
                        if let Some(tx) = self.sender.as_ref() {
                            // check whether play finished
                            if is_finished {
                                let buf_occupied =
                                    self.play_time.occupied_len.load(Ordering::Relaxed);
                                // play finished
                                if buf_occupied == 0 {
                                    // send finish event
                                    if let Err(e) = tx.try_send(Events::PlayFinished) {
                                        eprintln!("error when send event: {}", e);
                                    }
                                    // set state
                                    self.controller.stop();
                                    break;
                                }
                            }
                            // send current play time
                            let time = self.play_time.played_time();
                            let current_time = time.seconds as f64 + time.frac;
                            if current_time >= (last_sent_time + 0.1) {
                                last_sent_time = last_sent_time.max(current_time);
                                if let Err(e) = tx.try_send(Events::PlaytimeRefresh) {
                                    eprintln!("error when send event: {}", e);
                                }
                            }
                        }
                    }
                    ServiceState::Paused => {
                        self.controller.wait_if_paused();
                    }
                    ServiceState::Stopped => break,
                    ServiceState::Seek(t) => {
                        let r = self.music_decoder.format.seek(
                            SeekMode::Accurate,
                            SeekTo::Time {
                                time: t,
                                track_id: None,
                            },
                        );
                        match r {
                            Ok(s) => {
                                self.leftover_samples.clear();
                                self.play_time
                                    .decoded_len
                                    .store(s.actual_ts, Ordering::Relaxed);
                            }
                            Err(e) => eprintln!("error: {}", e),
                        }
                        self.controller.play();
                        continue;
                    }
                }

                // if have overflowed data, push first
                if !self.leftover_samples.is_empty() {
                    let written = self
                        .producer
                        .push_slice(self.leftover_samples.make_contiguous());
                    self.leftover_samples.drain(..written);
                }

                // if ringbuff is full, wait
                if self.producer.is_full() {
                    thread::sleep(Duration::from_millis(50));
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

    /// decode and process stream
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
