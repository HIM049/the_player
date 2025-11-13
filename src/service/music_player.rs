use rodio::{Decoder, OutputStream, Sink};
use std::{fs::File, io::BufReader};

use crate::service::music_service::music::Music;

/// Handle music decode and play
pub struct MusicPlayer {
    stream_handle: OutputStream,
    sink: Sink,
}

impl MusicPlayer {
    pub fn new() -> Result<Self, anyhow::Error> {
        let stream_handle = rodio::OutputStreamBuilder::open_default_stream()?;
        let sink = Sink::connect_new(&stream_handle.mixer());
        Ok(Self {
            stream_handle: stream_handle,
            sink: sink,
        })
    }

    pub fn append_file(&self, file: BufReader<File>) -> Result<(), anyhow::Error> {
        self.sink.append(Decoder::try_from(file)?);
        Ok(())
    }

    pub fn append_music(&self, music: &Music) -> Result<(), anyhow::Error> {
        self.append_file(BufReader::new(music.open_file()?))?;
        Ok(())
    }

    pub fn append_form_path(&self, path: String) -> Result<(), anyhow::Error> {
        let file = BufReader::new(File::open(path)?);
        self.append_file(file)?;
        Ok(())
    }

    pub fn play(&self) {
        self.sink.play();
    }

    pub fn pause(&self) {
        self.sink.pause();
    }

    pub fn clear(&self) {
        self.sink.clear();
    }
}
