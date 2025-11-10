use crate::service::{music::Music, music_player::MusicPlayer};


pub struct MusicService {
    player: MusicPlayer,
    play_queue: Vec<Music>,
    queue_index: usize,
}

impl MusicService {
    pub fn new() -> Self {
        Self { 
            player: MusicPlayer::new().unwrap(), 
            play_queue: vec![], 
            queue_index: 0
        }
    }

    pub fn append_music(&mut self, music: Music) {
        self.play_queue.push(music);
    }

    pub fn play() {
        
    }
}