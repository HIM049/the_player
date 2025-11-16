/// A struct for play state
#[derive(PartialEq, Clone, Copy)]
pub enum PlayState {
    Playing,
    Paused,
    Stopped,
}

pub static RINGBUF_SIZE: usize = 48000;
