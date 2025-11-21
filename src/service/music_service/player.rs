use atomic_float::AtomicF32;
use cpal::SampleRate;
use ringbuf::{storage::Heap, traits::Split};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, mpsc};

use crate::service::music_service::controller::Controller;
use crate::service::music_service::decoder::Decoder;
use crate::service::music_service::models::{self, Events, PlayState};
use crate::service::music_service::output::Output;
use crate::service::music_service::service::Service;
use crate::service::music_service::time::PlayTime;

// reInit for every new song
pub struct Player {
    // output device
    output: Output,
    // decode controller
    controller: Arc<Controller>,
    // current music track info
    // track: Track,
    play_time: Arc<PlayTime>,
    receiver: Arc<Receiver<Events>>,
}

impl Player {
    /// Create a new player
    /// Used to play a file, will create a decode thread and output thread.
    pub fn new(file_path: PathBuf, gain: Arc<AtomicF32>) -> Result<Self, anyhow::Error> {
        // setup ringbuf
        let rb = ringbuf::SharedRb::<Heap<f32>>::new(models::RINGBUF_SIZE);
        let (producer, consumer) = rb.split();

        // create channel
        let (tx, rx) = mpsc::channel::<Events>();

        // create atomic counter
        let decoded_len = Arc::new(AtomicU64::new(0));
        let buf_occupied = Arc::new(AtomicUsize::new(0));
        // decode file
        let decoded = Decoder::decode_from_path(file_path)?;
        // setup output
        let output = Output::new(
            consumer,
            SampleRate(decoded.sample_rate),
            gain.clone(),
            buf_occupied.clone(),
        )?;
        // create decoder controller
        let controller = Arc::new(Controller::new());
        // clone track data
        let track = decoded.format.default_track().unwrap().clone();

        // init play time
        let play_time = Arc::new(
            PlayTime::from_track(
                track,
                output.supported_config.sample_rate.0,
                decoded_len.clone(),
                buf_occupied.clone(),
            )
            .unwrap(),
        );

        // create and run service thread
        Service::new(decoded, producer, controller.clone(), play_time.clone())
            .subscribe(tx)
            .start_service()
            .unwrap();

        // return self
        Ok(Self {
            output,
            controller,
            play_time,
            receiver: Arc::new(rx),
        })
    }

    pub fn play_time(&self) -> &PlayTime {
        &self.play_time
    }

    /// Get occupied length
    // pub fn occupied_len(&self) -> usize {
    //     self.occupied_len.load(Ordering::Relaxed)
    // }

    pub fn receiver(&self) -> Arc<Receiver<Events>> {
        self.receiver.clone()
    }

    pub fn state(&self) -> PlayState {
        self.controller.state()
    }

    /// Start decode and output.
    pub fn play(&self) {
        self.controller.play();
        self.output.play();
    }

    /// Pause decode and output.
    pub fn pause(&self) {
        self.output.pause();
        self.controller.pause();
    }

    /// Stop decode thread and pause output (waiting for drop)
    pub fn stop(&self) {
        self.output.pause();
        self.controller.stop();
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.stop();
    }
}
