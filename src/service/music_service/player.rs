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
    pub fn new(file_path: PathBuf) -> Self {
        let capacity = 4096;
        let rb = ringbuf::SharedRb::<Heap<f32>>::new(capacity);
        let (producer, consumer) = rb.split();

        let decoded = Decoder::decode_from_path(file_path).unwrap();
        let output = Output::new(consumer, SampleRate(decoded.sample_rate)).unwrap();
        let controller = Arc::new(Controller::new());
        Decoder::start_decoder(
            decoded,
            producer,
            controller.clone(),
            output.supported_config.sample_rate,
        )
        .unwrap();

        Self { output, controller }
    }

    pub fn play(&self) {
        self.controller.play();
        self.output.play();
    }

    pub fn pause(&self) {
        self.output.pause();
        self.controller.pause();
    }
}
