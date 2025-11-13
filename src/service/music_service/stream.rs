use rubato::{FftFixedIn, Resampler};
use symphonia::core::audio::{AudioBufferRef, Signal};

pub struct Stream {
    resampler: FftFixedIn<f32>,
    input_rate: usize,
    output_rate: usize,
    channels: usize,
}

impl Stream {
    pub fn new(
        input_rate: usize,
        output_rate: usize,
        chunk_size: usize,
        channels: usize,
    ) -> Result<Self, anyhow::Error> {
        let r = FftFixedIn::<f32>::new(input_rate, output_rate, chunk_size, 1, channels)?;
        Ok(Self {
            resampler: r,
            input_rate,
            output_rate,
            channels,
        })
    }

    pub fn process(&mut self, input: &Vec<f32>) -> Vec<f32> {
        if self.input_rate == self.output_rate {
            return input.to_vec();
        }

        let frames = input.len() / self.channels;

        // split channels
        let mut channels_data = vec![Vec::with_capacity(frames); self.channels];
        for (i, &sample) in input.iter().enumerate() {
            channels_data[i % self.channels].push(sample);
        }

        let outputs = self
            .resampler
            .process(&channels_data, None)
            .expect("Resampling failed");

        // mix two channels
        let frames_out = outputs[0].len();
        let mut interleaved = Vec::with_capacity(frames_out * self.channels);
        for i in 0..frames_out {
            for ch in 0..self.channels {
                interleaved.push(outputs[ch][i]);
            }
        }

        interleaved
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
                        let sample = cow.chan(ch)[frame_idx] as f32 / 128.0;
                        sample_packet.push(sample)
                    }
                }
            }
            AudioBufferRef::S16(cow) => {
                sample_rate = cow.spec().rate;
                channels = cow.spec().channels.count();
                for frame_idx in 0..cow.frames() {
                    for ch in 0..channels {
                        let sample = cow.chan(ch)[frame_idx] as f32 / 32768.0;
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
                        let sample = raw_val as f32 / 8_388_608.0; // 2^23 - This was already correct for S24
                        sample_packet.push(sample)
                    }
                }
            }
            AudioBufferRef::S32(cow) => {
                sample_rate = cow.spec().rate;
                channels = cow.spec().channels.count();
                for frame_idx in 0..cow.frames() {
                    for ch in 0..channels {
                        let sample = cow.chan(ch)[frame_idx] as f32 / 2147483648.0;
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
