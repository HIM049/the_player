use std::sync::{Condvar, Mutex};

use symphonia::core::units::Time;

#[derive(PartialEq, Clone, Copy)]
pub enum ServiceState {
    Playing,
    Paused,
    Stopped,
    Seek(Time),
}

/// The decode thread controller
pub struct Controller {
    state: Mutex<ServiceState>,
    condvar: Condvar,
}

impl Controller {
    /// Create a new controller
    pub fn new() -> Self {
        Self {
            state: Mutex::new(ServiceState::Playing),
            condvar: Condvar::new(),
        }
    }

    /// Set state and notify to resume (play)
    pub fn play(&self) {
        let mut state = self.state.lock().unwrap();
        *state = ServiceState::Playing;
        self.condvar.notify_one();
    }

    /// Set state to pause
    pub fn pause(&self) {
        let mut state = self.state.lock().unwrap();
        *state = ServiceState::Paused;
    }

    /// Set state to stop
    pub fn stop(&self) {
        let mut state = self.state.lock().unwrap();
        *state = ServiceState::Stopped;
    }

    pub fn seek_to(&self, seek_to: Time) {
        let mut state = self.state.lock().unwrap();
        *state = ServiceState::Seek(seek_to);
    }

    pub fn state(&self) -> ServiceState {
        *self.state.lock().unwrap()
    }

    /// Pause thread when need
    pub fn wait_if_paused(&self) {
        let mut state_guard = self.state.lock().unwrap();
        while *state_guard == ServiceState::Paused {
            state_guard = self.condvar.wait(state_guard).unwrap();
        }
    }
}
