use anyhow::anyhow;
use std::fs::File;
use std::path::PathBuf;
use symphonia::core::codecs::{self, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

pub struct Decoder {
    pub sample_rate: u32,
    pub format: Box<dyn FormatReader>,
    pub decoder: Box<dyn codecs::Decoder>,
}

impl Decoder {
    /// Decode from a file path
    pub fn decode_from_path(file_path: PathBuf) -> Result<Self, anyhow::Error> {
        Ok(Self::decode_file(Box::new(File::open(file_path)?))?)
    }

    /// Decode from file
    pub fn decode_file(file: Box<File>) -> Result<Self, anyhow::Error> {
        let probe = symphonia::default::get_probe();
        let mss = MediaSourceStream::new(file, Default::default());
        let probed = probe.format(
            &Default::default(),
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?;

        let format = probed.format;
        let track = format
            .default_track()
            .ok_or_else(|| anyhow!("no track found"))?;
        let codec_params = track.codec_params.clone();
        let sample_rate = codec_params.sample_rate.unwrap_or(44100);

        let codecs = symphonia::default::get_codecs();
        let decoder = codecs.make(&codec_params, &DecoderOptions::default())?;

        Ok(Self {
            sample_rate,
            format,
            decoder,
        })
    }
}
