use crate::domain::Segment;
use crate::infra::models::DiarizationModelPaths;
use crate::ResomerError;
use async_trait::async_trait;
use hound::WavReader;
use sherpa_onnx::{
    FastClusteringConfig, LinearResampler, OfflineSpeakerDiarization,
    OfflineSpeakerDiarizationConfig, OfflineSpeakerSegmentationModelConfig,
    OfflineSpeakerSegmentationPyannoteModelConfig, SpeakerEmbeddingExtractorConfig,
};
use std::path::Path;

// Distance threshold for auto speaker-count clustering (used when
// num_clusters < 0). Per sherpa-onnx's own docs: "the smaller, the more
// clusters it will generate; the larger, the fewer clusters it will
// generate." The library default (0.5) tends to over-split a single speaker
// into several detected speakers when their tone/volume varies across a
// recording. 0.7 trades a little bit of under-splitting risk (rarely
// merging two *different* quiet/similar-sounding voices) for noticeably
// fewer false extra speakers, which matches this app's meeting use case
// better. Tune here if real recordings still misbehave in either direction.
const CLUSTERING_THRESHOLD: f32 = 0.7;

pub struct SherpaDiarizationEngine;

impl Default for SherpaDiarizationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SherpaDiarizationEngine {
    pub fn new() -> Self {
        Self
    }

    // Downmixes interleaved i16 samples to mono f32 in [-1.0, 1.0].
    fn to_mono_f32(samples: &[i16], channels: u16) -> Vec<f32> {
        if channels <= 1 {
            return samples.iter().map(|&s| s as f32 / 32768.0).collect();
        }

        let channels = channels as usize;
        samples
            .chunks(channels)
            .map(|frame| {
                let sum: i32 = frame.iter().map(|&s| s as i32).sum();
                (sum as f32 / frame.len() as f32) / 32768.0
            })
            .collect()
    }

    fn diarize_blocking(audio_path: &str) -> Result<Vec<Segment>, ResomerError> {
        let paths = DiarizationModelPaths::resolve()?;

        let mut reader = WavReader::open(Path::new(audio_path))
            .map_err(|e| ResomerError::Diarization(format!("Failed to open WAV: {}", e)))?;

        let spec = reader.spec();
        let samples: Vec<i16> = reader
            .samples::<i16>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ResomerError::Diarization(format!("Failed to read samples: {}", e)))?;

        if samples.is_empty() {
            return Ok(vec![]);
        }

        let mono = Self::to_mono_f32(&samples, spec.channels);

        let config = OfflineSpeakerDiarizationConfig {
            segmentation: OfflineSpeakerSegmentationModelConfig {
                pyannote: OfflineSpeakerSegmentationPyannoteModelConfig {
                    model: Some(paths.segmentation.to_string_lossy().to_string()),
                },
                ..Default::default()
            },
            embedding: SpeakerEmbeddingExtractorConfig {
                model: Some(paths.embedding.to_string_lossy().to_string()),
                ..Default::default()
            },
            clustering: FastClusteringConfig {
                num_clusters: -1, // auto-detect: we don't know the speaker count upfront
                threshold: CLUSTERING_THRESHOLD,
            },
            ..Default::default()
        };

        let sd = OfflineSpeakerDiarization::create(&config).ok_or_else(|| {
            ResomerError::Diarization("No se pudo inicializar el motor de diarización".to_string())
        })?;

        let model_rate = sd.sample_rate();
        let samples_for_model = if model_rate != spec.sample_rate as i32 {
            let resampler = LinearResampler::create(spec.sample_rate as i32, model_rate)
                .ok_or_else(|| {
                    ResomerError::Diarization("No se pudo inicializar el resampler".to_string())
                })?;
            resampler.resample(&mono, true)
        } else {
            mono
        };

        let result = sd.process(&samples_for_model).ok_or_else(|| {
            ResomerError::Diarization("El procesamiento de diarización falló".to_string())
        })?;

        Ok(result
            .sort_by_start_time()
            .into_iter()
            .map(|s| Segment {
                start: s.start,
                end: s.end,
                speaker: format!("Speaker-{}", s.speaker),
            })
            .collect())
    }
}

#[async_trait]
impl crate::domain::DiarizationEngine for SherpaDiarizationEngine {
    async fn diarize(&self, audio_path: &str) -> Result<Vec<Segment>, ResomerError> {
        let audio_path = audio_path.to_string();
        tokio::task::spawn_blocking(move || Self::diarize_blocking(&audio_path))
            .await
            .map_err(|e| ResomerError::Diarization(format!("Diarization task panicked: {}", e)))?
    }
}
