use anyhow::anyhow;
use cpal::SampleRate;
use cpal::traits::{HostTrait, StreamTrait};
use ringbuf::{HeapCons, traits::Consumer};
use rodio::DeviceTrait;

pub struct Output {
    pub host: cpal::Host,
    pub device: cpal::Device,
    pub supported_config: cpal::StreamConfig,
    pub stream: cpal::Stream,
}

impl Output {
    pub fn new(
        mut consumer: HeapCons<f32>,
        target_sample_rate: SampleRate,
    ) -> Result<Self, anyhow::Error> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(anyhow!("no output device available"))?;

        // init config
        let supported_config;
        let mut supported_configs_range = device.supported_output_configs()?;
        let support_config_range = supported_configs_range.find(|config| {
            config.min_sample_rate() <= target_sample_rate
                && target_sample_rate <= config.max_sample_rate()
        });
        if let Some(config) = support_config_range {
            supported_config = config.with_sample_rate(target_sample_rate).config();
        } else {
            let mut supported_configs_range = device.supported_output_configs()?;
            supported_config = supported_configs_range
                .next()
                .ok_or(anyhow!("no supported config"))?
                .with_max_sample_rate()
                .config();
        }

        let stream = device
            .build_output_stream(
                &supported_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    consumer.pop_slice(data);
                },
                move |err| {
                    eprintln!("error: {}", err);
                },
                None,
            )
            .unwrap();

        Ok(Self {
            host: host,
            device: device,
            supported_config: supported_config,
            stream: stream,
        })
    }

    pub fn play(&self) {
        self.stream.play().unwrap();
    }

    pub fn pause(&self) {
        self.stream.pause().unwrap();
    }
}
