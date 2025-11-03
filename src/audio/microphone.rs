use cpal::traits::HostTrait as _;
use cpal::{Stream, SupportedStreamConfig};
use rodio::DeviceTrait as _;
use std::sync::mpsc::Sender;
use std::time::SystemTime;

pub fn microphone_stream<T>(
    output: Sender<(Vec<T>, SystemTime)>,
    config: SupportedStreamConfig,
) -> anyhow::Result<Stream>
where
    T: cpal::SizedSample + Send + 'static,
{
    let host = cpal::default_host();
    let device = host.default_input_device().unwrap();
    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[T], _: &_| {
            if let Err(e) = output.send((data.to_vec(), SystemTime::now())) {
                log::error!("{}", e)
            }
        },
        |e| log::error!("{}", e),
        None,
    )?;

    Ok(stream)
}
