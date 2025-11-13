use std::sync::{Condvar, Mutex};

use crate::service::music_service::models::PlayState;

pub struct Controller {
    state: Mutex<PlayState>,
    condvar: Condvar,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(PlayState::Playing),
            condvar: Condvar::new(),
        }
    }

    pub fn play(&self) {
        let mut state = self.state.lock().unwrap();
        *state = PlayState::Playing;
        self.condvar.notify_one();
    }

    pub fn is_playing(&self) -> bool {
        let state = self.state.lock().unwrap();
        *state == PlayState::Playing
    }

    pub fn pause(&self) {
        let mut state = self.state.lock().unwrap();
        *state = PlayState::Paused;
    }

    pub fn is_paused(&self) -> bool {
        let state = self.state.lock().unwrap();
        *state == PlayState::Paused
    }

    pub fn stop(&self) {
        let mut state = self.state.lock().unwrap();
        *state = PlayState::Stopped;
    }

    pub fn is_stopped(&self) -> bool {
        let state = self.state.lock().unwrap();
        *state == PlayState::Stopped
    }

    pub fn condvar(&self) -> &Condvar {
        &self.condvar
    }
    pub fn wait_if_paused(&self) {
        let mut state_guard = self.state.lock().unwrap();
        while *state_guard == PlayState::Paused {
            state_guard = self.condvar.wait(state_guard).unwrap();
        }
    }
}
