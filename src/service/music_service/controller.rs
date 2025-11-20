use std::sync::{Condvar, Mutex};

use crate::service::music_service::models::PlayState;

/// The decode thread controller
pub struct Controller {
    state: Mutex<PlayState>,
    condvar: Condvar,
}

impl Controller {
    /// Create a new controller
    pub fn new() -> Self {
        Self {
            state: Mutex::new(PlayState::Playing),
            condvar: Condvar::new(),
        }
    }

    /// Set state and notify to resume (play)
    pub fn play(&self) {
        let mut state = self.state.lock().unwrap();
        *state = PlayState::Playing;
        self.condvar.notify_one();
    }

    /// Get current status
    pub fn is_playing(&self) -> bool {
        let state = self.state.lock().unwrap();
        *state == PlayState::Playing
    }

    /// Set state to pause
    pub fn pause(&self) {
        let mut state = self.state.lock().unwrap();
        *state = PlayState::Paused;
    }

    /// Get current status
    pub fn is_paused(&self) -> bool {
        let state = self.state.lock().unwrap();
        *state == PlayState::Paused
    }

    /// Set state to stop
    pub fn stop(&self) {
        let mut state = self.state.lock().unwrap();
        *state = PlayState::Stopped;
    }

    /// Get current status
    pub fn is_stopped(&self) -> bool {
        let state = self.state.lock().unwrap();
        *state == PlayState::Stopped
    }

    /// Get condvar
    pub fn condvar(&self) -> &Condvar {
        &self.condvar
    }

    pub fn state(&self) -> PlayState {
        *self.state.lock().unwrap()
    }

    /// Pause thread when need
    pub fn wait_if_paused(&self) {
        let mut state_guard = self.state.lock().unwrap();
        while *state_guard == PlayState::Paused {
            state_guard = self.condvar.wait(state_guard).unwrap();
        }
    }
}
