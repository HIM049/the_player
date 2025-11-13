use std::path::PathBuf;

use crate::service::music_service::{music::Music, player::Player};

pub struct Core {
    pub player: Option<Player>,
    pub current: Option<Music>,
    // queue: Vec<Music>,
}

impl Core {
    pub fn new() -> Self {
        Self {
            player: None,
            current: None,
        }
    }

    pub fn append(&mut self, path: PathBuf) {
        self.player = Some(Player::new(path.clone()));
        self.current = Some(Music::from_path(path).unwrap());
        self.play();
    }

    pub fn play(&self) {
        if let Some(p) = self.player.as_ref() {
            p.play();
        }
    }

    pub fn pause(&self) {
        if let Some(p) = self.player.as_ref() {
            p.pause();
        }
    }
}
