use std::{
    path::PathBuf,
    sync::{Arc, atomic::Ordering},
};

use atomic_float::AtomicF32;

use crate::service::music_service::{models::PlayState, music::Music, player::Player};

pub struct Core {
    pub player: Option<Player>,
    pub current: Option<Music>,
    gain: Arc<AtomicF32>,
    state: PlayState,
    // queue: Vec<Music>,
}

impl Core {
    pub fn new() -> Self {
        Self {
            player: None,
            current: None,
            gain: Arc::new(AtomicF32::new(0.5)),
            state: PlayState::Stopped,
        }
    }

    pub fn append(&mut self, path: PathBuf) -> Result<(), anyhow::Error> {
        self.player = Some(Player::new(path.clone(), self.gain.clone())?);
        self.current = Some(Music::from_path(path)?);
        self.play();
        Ok(())
    }

    pub fn play(&mut self) {
        self.state = PlayState::Playing;
        if let Some(p) = self.player.as_ref() {
            p.play();
        }
    }

    pub fn pause(&mut self) {
        self.state = PlayState::Paused;
        if let Some(p) = self.player.as_ref() {
            p.pause();
        }
    }

    pub fn stop(&mut self) {
        self.state = PlayState::Stopped;
        self.player = None;
        self.current = None
    }

    pub fn get_state(&self) -> PlayState {
        self.state
    }

    pub fn set_gain(&self, new_value: f32) {
        self.gain.store(new_value, Ordering::Relaxed);
    }
}
