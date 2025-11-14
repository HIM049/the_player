use cpal::SampleRate;
use ringbuf::{storage::Heap, traits::Split};
use std::path::PathBuf;
use std::sync::Arc;

use crate::service::music_service::controller::Controller;
use crate::service::music_service::decoder::Decoder;
use crate::service::music_service::output::Output;

// reInit for every new song
pub struct Player {
    output: Output,
    controller: Arc<Controller>,
}

impl Player {
    /// Create a new player
    /// Used to play a file, create a decode thread and output thread.
    pub fn new(file_path: PathBuf) -> Result<Self, anyhow::Error> {
        // setup ringbuf
        let capacity = 48000;
        let rb = ringbuf::SharedRb::<Heap<f32>>::new(capacity);
        let (producer, consumer) = rb.split();

        // decode file
        let decoded = Decoder::decode_from_path(file_path)?;
        // setup output
        let output = Output::new(consumer, SampleRate(decoded.sample_rate))?;
        // create decoder controller
        let controller = Arc::new(Controller::new());
        // create and run decode thread
        Decoder::start_decoder(
            decoded,
            producer,
            controller.clone(),
            output.supported_config.sample_rate,
        )?;

        Ok(Self { output, controller })
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
