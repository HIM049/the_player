use atomic_float::AtomicF32;
use cpal::SampleRate;
use ringbuf::{storage::Heap, traits::Split};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use symphonia::core::formats::Track;
use symphonia::core::units::Time;

use crate::service::music_service::controller::Controller;
use crate::service::music_service::decoder::Decoder;
use crate::service::music_service::models;
use crate::service::music_service::output::Output;

// reInit for every new song
pub struct Player {
    output: Output,
    controller: Arc<Controller>,
    decoded_len: Arc<AtomicU64>,
    buf_occupied: Arc<AtomicUsize>,
    track: Track,
}

impl Player {
    /// Create a new player
    /// Used to play a file, create a decode thread and output thread.
    pub fn new(file_path: PathBuf, gain: Arc<AtomicF32>) -> Result<Self, anyhow::Error> {
        // setup ringbuf
        let rb = ringbuf::SharedRb::<Heap<f32>>::new(models::RINGBUF_SIZE);
        let (producer, consumer) = rb.split();

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

        // create and run decode thread
        Decoder::start_decoder(
            decoded,
            producer,
            controller.clone(),
            output.supported_config.sample_rate,
            decoded_len.clone(),
        )?;

        // return self
        Ok(Self {
            output,
            controller,
            decoded_len,
            buf_occupied,
            track,
        })
    }

    /// Calculate time by length of sample
    pub fn calc_time(&self, ts: u64) -> Option<Time> {
        let duration = self.track.codec_params.time_base?.calc_time(ts);
        Some(duration)
    }

    /// Get music played time
    pub fn played_time(&self) -> Option<Time> {
        let occupied = self.buf_occupied.load(Ordering::Relaxed);
        let latency_samples = (((occupied as u32 / self.track.codec_params.channels?.count() as u32)
            as f32
            / self.output.supported_config.sample_rate.0 as f32)
            * self.track.codec_params.sample_rate? as f32) as u64;

        let played_len = self
            .decoded_len
            .load(Ordering::Relaxed)
            .saturating_sub(latency_samples);
        self.calc_time(played_len)
    }

    /// Get music langth time
    pub fn duration(&self) -> Option<Time> {
        let n_frames = self.track.codec_params.n_frames?;
        self.calc_time(n_frames)
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
