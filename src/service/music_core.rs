use std::{f32::consts::PI, thread::sleep, time::Duration};

use anyhow::anyhow;
use cpal::traits::{HostTrait, StreamTrait};
use rodio::DeviceTrait;


pub struct MusicCore {
    host: cpal::Host,
    device: cpal::Device,
    supported_config: cpal::SupportedStreamConfig,
    stream: cpal::Stream
}

impl MusicCore {
    pub fn new() -> Result<Self, anyhow::Error> {
        let host = cpal::default_host();
        let device = host.default_output_device()
            .ok_or(anyhow!("no output device available"))?;
        
        let mut supported_configs_range = device.supported_output_configs()?;
        let supported_config = supported_configs_range.next()
            .ok_or(anyhow!("no supported config"))?
            .with_max_sample_rate();

        Ok(Self { 
            host: host, 
            device: device, 
            supported_config: supported_config,
            stream: todo!(), 
        })
    }
    pub fn some() {
        let host = cpal::default_host();
        let device = host.default_output_device()
            .expect("no output device available");
        let mut supported_configs_range = device.supported_output_configs()
            .expect("error while querying configs");
        let supported_config = supported_configs_range.next()
            .expect("no supported config")
            .with_max_sample_rate();

        let config = supported_config.config();

        let stream = device.build_output_stream(
            &config, 
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let sample_rate: f32 = 44100.0;
                let frequency: f32 = 440.0; 
                let phase_increment: f32 = (2.0 * PI * frequency) / sample_rate;
                for (index, sample) in data.iter_mut().enumerate() {

                    let current_phase = index as f32 * phase_increment;
                    let value = current_phase.sin();
                    
                    *sample = value * 0.5;
                }
            },
            move |err| {
                eprintln!("an error occurred on the output audio stream: {}", err);
            },
            None
        ).unwrap();

        stream.play().unwrap();
        // stream.pause()
        sleep(Duration::from_secs(60));
    }
}