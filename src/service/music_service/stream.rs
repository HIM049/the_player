use symphonia::core::audio::{AudioBufferRef, Signal};

pub struct Stream;

impl Stream {
    pub fn resample_stereo(samples: Vec<f32>, from_rate: u32, to_rate: u32) -> Vec<f32> {
        assert!(
            samples.len() % 2 == 0,
            "stereo data length should be an even"
        );

        let (left, right): (Vec<f32>, Vec<f32>) =
            samples.chunks(2).map(|chunk| (chunk[0], chunk[1])).unzip();

        let left_resampled = Self::resample_mono(&left, from_rate, to_rate);
        let right_resampled = Self::resample_mono(&right, from_rate, to_rate);

        let mut interleaved = Vec::with_capacity(left_resampled.len() * 2);
        for (l, r) in left_resampled.into_iter().zip(right_resampled) {
            interleaved.push(l);
            interleaved.push(r);
        }

        interleaved
    }

    pub fn resample_mono(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
        let ratio = to_rate as f32 / from_rate as f32;
        let new_len = (samples.len() as f32 * ratio).round() as usize;
        let mut resampled = Vec::with_capacity(new_len);

        for i in 0..new_len {
            let pos = i as f32 / ratio;
            let idx = pos.floor() as usize;
            let frac = pos.fract();

            let s0 = samples.get(idx).copied().unwrap_or(0.0);
            let s1 = samples.get(idx + 1).copied().unwrap_or(0.0);
            resampled.push(s0 * (1.0 - frac) + s1 * frac);
        }

        resampled
    }

    pub fn transfer_to_f32(buff: AudioBufferRef) -> (Vec<f32>, u32, usize) {
        let mut sample_packet = vec![];
        let sample_rate: u32;
        let channels: usize;
        match buff {
            AudioBufferRef::U8(cow) => {
                sample_rate = cow.spec().rate;
                channels = cow.spec().channels.count();
                for frame_idx in 0..cow.frames() {
                    for ch in 0..channels {
                        let raw = cow.chan(ch)[frame_idx];
                        let norm = raw as f32 / u8::MAX as f32;
                        let sample = norm * 2.0 - 1.0;
                        sample_packet.push(sample)
                    }
                }
            }
            AudioBufferRef::U16(cow) => {
                sample_rate = cow.spec().rate;
                channels = cow.spec().channels.count();
                for frame_idx in 0..cow.frames() {
                    for ch in 0..channels {
                        let raw = cow.chan(ch)[frame_idx];
                        let norm = raw as f32 / u16::MAX as f32;
                        let sample = norm * 2.0 - 1.0;
                        sample_packet.push(sample)
                    }
                }
            }
            AudioBufferRef::U24(cow) => {
                sample_rate = cow.spec().rate;
                channels = cow.spec().channels.count();
                for frame_idx in 0..cow.frames() {
                    for ch in 0..channels {
                        let raw = cow.chan(ch)[frame_idx].0;
                        let norm = raw as f32 / 16_777_215.0;
                        let sample = norm * 2.0 - 1.0;
                        sample_packet.push(sample)
                    }
                }
            }
            AudioBufferRef::U32(cow) => {
                sample_rate = cow.spec().rate;
                channels = cow.spec().channels.count();
                for frame_idx in 0..cow.frames() {
                    for ch in 0..channels {
                        let raw = cow.chan(ch)[frame_idx];
                        let norm = raw as f32 / u32::MAX as f32;
                        let sample = norm * 2.0 - 1.0;
                        sample_packet.push(sample)
                    }
                }
            }
            AudioBufferRef::S8(cow) => {
                sample_rate = cow.spec().rate;
                channels = cow.spec().channels.count();
                for frame_idx in 0..cow.frames() {
                    for ch in 0..channels {
                        let sample = cow.chan(ch)[frame_idx] as f32 / i8::MAX as f32;
                        sample_packet.push(sample)
                    }
                }
            }
            AudioBufferRef::S16(cow) => {
                sample_rate = cow.spec().rate;
                channels = cow.spec().channels.count();
                for frame_idx in 0..cow.frames() {
                    for ch in 0..channels {
                        let sample = cow.chan(ch)[frame_idx] as f32 / i16::MAX as f32;
                        sample_packet.push(sample)
                    }
                }
            }
            AudioBufferRef::S24(cow) => {
                sample_rate = cow.spec().rate;
                channels = cow.spec().channels.count();
                for frame_idx in 0..cow.frames() {
                    for ch in 0..channels {
                        let raw_val = cow.chan(ch)[frame_idx].0;
                        let sample = raw_val as f32 / 8_388_608.0; // 2^23
                        sample_packet.push(sample)
                    }
                }
            }
            AudioBufferRef::S32(cow) => {
                sample_rate = cow.spec().rate;
                channels = cow.spec().channels.count();
                for frame_idx in 0..cow.frames() {
                    for ch in 0..channels {
                        let sample = cow.chan(ch)[frame_idx] as f32 / i32::MAX as f32;
                        sample_packet.push(sample)
                    }
                }
            }
            AudioBufferRef::F32(cow) => {
                sample_rate = cow.spec().rate;
                channels = cow.spec().channels.count();
                for frame_idx in 0..cow.frames() {
                    for ch in 0..channels {
                        let sample = cow.chan(ch)[frame_idx];
                        sample_packet.push(sample);
                    }
                }
            }
            AudioBufferRef::F64(cow) => {
                sample_rate = cow.spec().rate;
                channels = cow.spec().channels.count();
                for frame_idx in 0..cow.frames() {
                    for ch in 0..channels {
                        let sample = cow.chan(ch)[frame_idx] as f32;
                        sample_packet.push(sample);
                    }
                }
            }
        }
        return (sample_packet, sample_rate, channels);
    }
}
