use std::{
    path::PathBuf,
    sync::{Arc, atomic::Ordering},
};

use atomic_float::AtomicF32;

use crate::service::music_service::{models::PlayState, music::Music, player::Player};

pub struct Core {
    player: Option<Player>,
    current: Option<Music>,
    gain: Arc<AtomicF32>,
    state: PlayState,
    // queue: Vec<Music>,
}

impl Core {
    /// Create a new core
    pub fn new() -> Self {
        Self {
            player: None,
            current: None,
            gain: Arc::new(AtomicF32::new(1.0)),
            state: PlayState::Stopped,
        }
    }

    pub fn player(&self) -> Option<&Player> {
        self.player.as_ref()
    }

    pub fn current(&self) -> Option<&Music> {
        self.current.as_ref()
    }

    // TODO: move to list struct
    /// Append a new song to core
    pub fn append(&mut self, path: PathBuf) -> Result<(), anyhow::Error> {
        self.player = Some(Player::new(path.clone(), self.gain.clone())?);
        self.current = Some(Music::from_path(path)?);
        self.play();
        Ok(())
    }

    /// Control core start/continue current play
    pub fn play(&mut self) {
        self.state = PlayState::Playing;
        if let Some(p) = self.player.as_ref() {
            p.play();
        }
    }

    /// Control core pause current play
    pub fn pause(&mut self) {
        self.state = PlayState::Paused;
        if let Some(p) = self.player.as_ref() {
            p.pause();
        }
    }

    /// Control core stop current play
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
