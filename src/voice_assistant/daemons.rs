use std::{
    sync::{mpsc::Receiver, Arc, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, SystemTime},
};

use oww_rs::oww::{OwwModel, OWW_MODEL_CHUNK_SIZE};
use voice_activity_detector_silero_v5::VoiceActivityDetector;

pub const SILERO_SAMPLE_RATE: u32 = 16000;
pub const SILERO_CHUNK_SIZE: usize = 512;
pub const OWW_SAMPLE_RATE: u32 = 16000;

pub fn vad(
    microphone: Receiver<(Vec<i16>, SystemTime)>,
    probability: Arc<Mutex<f32>>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut vad = VoiceActivityDetector::builder()
            .chunk_size(SILERO_CHUNK_SIZE)
            .sample_rate(SILERO_SAMPLE_RATE)
            .build()
            .unwrap();

        loop {
            let (data, time): (Vec<i16>, SystemTime) = microphone.recv().unwrap();
            if !SystemTime::now()
                .duration_since(time)
                .unwrap()
                .saturating_sub(Duration::from_millis(500))
                .is_zero()
            {
                dbg!("Skipped");
                continue;
            }
            let prob = vad.predict(data);
            let mut mutex_handle = probability.lock().unwrap();
            *mutex_handle = prob;
        }
    })
}

pub fn oww(
    microphone: Receiver<(Vec<f32>, SystemTime)>,
    probability: Arc<Mutex<f32>>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut oww =
            OwwModel::new(oww_rs::config::SpeechUnlockType::OpenWakeWordAlexa, 0.1).unwrap();

        let mut buffer = vec![];
        loop {
            let (data, time): (Vec<f32>, SystemTime) = microphone.recv().unwrap();
            let now = SystemTime::now();
            if !now
                .duration_since(time)
                .unwrap()
                .saturating_sub(Duration::from_millis(500))
                .is_zero()
            {
                dbg!("Skipped");
                continue;
            }

            buffer.extend(data);
            if buffer.len() < OWW_MODEL_CHUNK_SIZE {
                continue;
            }

            let detection = oww.detection(buffer[..OWW_MODEL_CHUNK_SIZE].to_vec());
            let mut mutex_handle = probability.lock().unwrap();
            *mutex_handle = detection.probability;
            drop(mutex_handle);
            buffer.clear();
        }
    })
}
