use std::{
    fs::File,
    io::{self, BufWriter},
    sync::{Arc, Mutex},
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BuildStreamError, DefaultStreamConfigError, FromSample, PlayStreamError, Sample, Stream,
    SupportedStreamConfig,
};
use log::error;
use tempfile::NamedTempFile;
use thiserror::Error;

pub struct Recording {
    writer: WavWriterHandle,
    stream: Stream,
    file: NamedTempFile,
}

impl Recording {
    pub fn start(/*is_vad: bool*/) -> Result<Self, StartRecordingError> {
        let file = NamedTempFile::new()?;
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or(StartRecordingError::DefaultInputDevice)?;
        let mut config = device.default_input_config()?;
        // if is_vad {
        //     config = SupportedStreamConfig::new(
        //         1,
        //         cpal::SampleRate(16000),
        //         *config.buffer_size(),
        //         cpal::SampleFormat::I16,
        //     );
        // }
        let spec = wav_spec_from_config(&config);
        let writer = hound::WavWriter::create(&file, spec)?;
        let writer = Arc::new(std::sync::Mutex::new(Some(writer)));
        let writer_2 = writer.clone();

        let stream = match config.sample_format() {
            cpal::SampleFormat::I8 => device.build_input_stream(
                &config.into(),
                move |data, _: &_| write_input_data::<i8, i8>(data, &writer_2),
                |e| error!("{}", e),
                None,
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data, _: &_| write_input_data::<i16, i16>(data, &writer_2),
                |e| error!("{}", e),
                None,
            )?,
            cpal::SampleFormat::I32 => device.build_input_stream(
                &config.into(),
                move |data, _: &_| write_input_data::<i32, i32>(data, &writer_2),
                |e| error!("{}", e),
                None,
            )?,
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data, _: &_| write_input_data::<f32, f32>(data, &writer_2),
                |e| error!("{}", e),
                None,
            )?,
            sample_format => Err(StartRecordingError::UnsupportedSampleFormat(sample_format))?,
        };

        stream.play()?;
        Ok(Self {
            writer,
            stream,
            file,
        })
    }

    pub fn stop(self) -> Result<NamedTempFile, StopRecordingError> {
        drop(self.stream);
        self.writer.lock().unwrap().take().unwrap().finalize()?;
        Ok(self.file)
    }
}

fn sample_format(format: cpal::SampleFormat) -> hound::SampleFormat {
    if format.is_float() {
        hound::SampleFormat::Float
    } else {
        hound::SampleFormat::Int
    }
}

fn wav_spec_from_config(config: &cpal::SupportedStreamConfig) -> hound::WavSpec {
    hound::WavSpec {
        channels: config.channels() as _,
        sample_rate: config.sample_rate().0 as _,
        bits_per_sample: (config.sample_format().sample_size() * 8) as _,
        sample_format: sample_format(config.sample_format()),
    }
}

type WavWriterHandle = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;

fn write_input_data<T, U>(input: &[T], writer: &WavWriterHandle)
where
    T: Sample,
    U: Sample + hound::Sample + FromSample<T>,
{
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            for &sample in input.iter() {
                let sample: U = U::from_sample(sample);
                writer.write_sample(sample).ok();
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum StartRecordingError {
    #[error("Temporary file creation error")]
    TemporaryFile(#[from] io::Error),
    #[error("Cannot get default input device")]
    DefaultInputDevice,
    #[error(transparent)]
    DefaultStreamConfig(#[from] DefaultStreamConfigError),
    #[error(transparent)]
    Hound(#[from] hound::Error),
    #[error(transparent)]
    BuildStream(#[from] BuildStreamError),
    #[error("Unsupported sample format `{0}`")]
    UnsupportedSampleFormat(cpal::SampleFormat),
    #[error(transparent)]
    PlayStream(#[from] PlayStreamError),
}

#[derive(Error, Debug)]
pub enum StopRecordingError {
    #[error(transparent)]
    Hound(#[from] hound::Error),
}
