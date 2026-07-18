use crate::ResomerError;
use crate::domain::AudioRecorder;
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use hound::WavWriter;
use std::sync::Mutex;
use cpal::traits::StreamTrait;

pub struct SystemAudioRecorder {
    stream: Arc<std::sync::Mutex<Option<cpal::Stream>>>,
    is_recording: Arc<AtomicBool>,
}

impl SystemAudioRecorder {
    pub fn new() -> Result<Self, ResomerError> {
        Ok(Self {
            stream: Arc::new(Mutex::new(None)),
            is_recording: Arc::new(AtomicBool::new(false)),
        })
    }

    fn get_recording_stream(
        output_path: &str,
        is_recording: Arc<AtomicBool>,
    ) -> Result<cpal::Stream, ResomerError> {
        use cpal::traits::{DeviceTrait, HostTrait};

        let host = cpal::default_host();

        // Try to find a system audio input device
        // On macOS, system audio would be captured from the default input
        let device = host
            .default_input_device()
            .ok_or(ResomerError::RecordingError(
                "No input device found for system audio".to_string(),
            ))?;

        let supported_config = device
            .default_input_config()
            .map_err(|e| ResomerError::RecordingError(format!("Config error: {}", e)))?;

        let config = supported_config.config();
        let sample_rate = config.sample_rate;
        let channels = config.channels;

        // Create WAV writer for stereo system audio
        let spec = hound::WavSpec {
            channels: channels.max(2), // At least stereo
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let writer = Arc::new(Mutex::new(WavWriter::create(output_path, spec).map_err(
            |e| ResomerError::RecordingError(format!("WAV creation failed: {}", e)),
        )?));

        let writer_clone = writer.clone();

        // Build the stream with proper sample format handling
        let stream = match supported_config.sample_format() {
            cpal::SampleFormat::F32 => device
                .build_input_stream(
                    config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if is_recording.load(Ordering::Relaxed) {
                            if let Ok(mut w) = writer_clone.try_lock() {
                                for &sample in data {
                                    let sample_i16 = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
                                    let _ = w.write_sample(sample_i16);
                                }
                            }
                        }
                    },
                    |_err| {},
                    None,
                )
                .map_err(|e| ResomerError::RecordingError(format!("Stream build failed: {}", e)))?,
            cpal::SampleFormat::I16 => device
                .build_input_stream(
                    config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if is_recording.load(Ordering::Relaxed) {
                            if let Ok(mut w) = writer_clone.try_lock() {
                                for &sample in data {
                                    let _ = w.write_sample(sample);
                                }
                            }
                        }
                    },
                    |_err| {},
                    None,
                )
                .map_err(|e| ResomerError::RecordingError(format!("Stream build failed: {}", e)))?,
            cpal::SampleFormat::U16 => device
                .build_input_stream(
                    config,
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        if is_recording.load(Ordering::Relaxed) {
                            if let Ok(mut w) = writer_clone.try_lock() {
                                for &sample in data {
                                    let sample_i16 = ((sample as i32) - 32768) as i16;
                                    let _ = w.write_sample(sample_i16);
                                }
                            }
                        }
                    },
                    |_err| {},
                    None,
                )
                .map_err(|e| ResomerError::RecordingError(format!("Stream build failed: {}", e)))?,
            _ => {
                return Err(ResomerError::RecordingError(
                    "Unsupported sample format".to_string(),
                ))
            }
        };

        Ok(stream)
    }
}

#[async_trait]
impl AudioRecorder for SystemAudioRecorder {
    async fn start_recording(&self, output_path: &str) -> Result<(), ResomerError> {
        // Create output directory if needed
        if let Some(parent) = std::path::Path::new(output_path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ResomerError::RecordingError(format!("Dir creation failed: {}", e))
                })?;
            }
        }

        let stream = Self::get_recording_stream(output_path, self.is_recording.clone())?;
        stream
            .play()
            .map_err(|e| ResomerError::RecordingError(format!("Stream play failed: {}", e)))?;

        self.is_recording.store(true, Ordering::Relaxed);
        *self.stream.lock().unwrap() = Some(stream);

        Ok(())
    }

    async fn stop_recording(&self) -> Result<(), ResomerError> {
        self.is_recording.store(false, Ordering::Relaxed);

        if let Some(stream) = self.stream.lock().unwrap().take() {
            drop(stream);
        }

        Ok(())
    }

    async fn pause_recording(&self) -> Result<(), ResomerError> {
        self.is_recording.store(false, Ordering::Relaxed);
        if let Some(stream) = self.stream.lock().unwrap().as_ref() {
            stream
                .pause()
                .map_err(|e| ResomerError::RecordingError(format!("Pause failed: {}", e)))?;
        }
        Ok(())
    }
}
