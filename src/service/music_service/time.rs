use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};

use symphonia::core::{
    formats::Track,
    units::{Time, TimeBase},
};

pub struct PlayTime {
    pub time_base: TimeBase,
    pub n_frames: u64,
    pub duration_sec: u64,
    pub channels: usize,
    pub sample_rate: u32,
    pub device_sample_rate: u32,
    // decoded frames length
    pub decoded_len: Arc<AtomicU64>,
    // ringbuf occupied length
    pub occupied_len: Arc<AtomicUsize>,
}

impl PlayTime {
    pub fn new(
        time_base: TimeBase,
        n_frames: u64,
        channels: usize,
        sample_rate: u32,
        device_sample_rate: u32,
        decoded_len: Arc<AtomicU64>,
        occupied_len: Arc<AtomicUsize>,
    ) -> Self {
        let duration_sec = time_base.calc_time(n_frames).seconds;
        Self {
            time_base,
            n_frames,
            duration_sec,
            channels,
            sample_rate,
            device_sample_rate,
            decoded_len,
            occupied_len,
        }
    }

    pub fn from_track(
        track: Track,
        device_sample_rate: u32,
        decoded_len: Arc<AtomicU64>,
        occupied_len: Arc<AtomicUsize>,
    ) -> Option<Self> {
        Some(Self::new(
            track.codec_params.time_base?,
            track.codec_params.n_frames?,
            track.codec_params.channels?.count(),
            track.codec_params.sample_rate?,
            device_sample_rate,
            decoded_len,
            occupied_len,
        ))
    }

    pub fn duration(&self) -> Time {
        self.time_base.calc_time(self.n_frames)
    }

    pub fn duration_sec(&self) -> u64 {
        self.duration_sec
    }

    /// Get music played time
    pub fn played_time(&self) -> Time {
        let occupied = self.occupied_len.load(Ordering::Relaxed);
        let latency_samples = (((occupied as u32 / self.channels as u32) as f32
            / self.device_sample_rate as f32)
            * self.sample_rate as f32) as u64;

        let played_len = self
            .decoded_len
            .load(Ordering::Relaxed)
            .saturating_sub(latency_samples);
        self.time_base.calc_time(played_len)
    }

    /// Get music played time
    pub fn played_sec(&self) -> u64 {
        self.played_time().seconds
    }
}
