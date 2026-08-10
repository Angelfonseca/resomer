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
//
// This threshold ONLY applies when the caller doesn't know the speaker count.
// When the user tells us how many participants there are, we pin
// `num_clusters` to that exact number and the threshold is ignored — which is
// by far the most reliable way to avoid phantom speakers.
const CLUSTERING_THRESHOLD: f32 = 0.7;

// Post-processing guardrails for the *auto* (unknown speaker count) path.
// Even with a tuned threshold, auto clustering occasionally spawns a phantom
// speaker made up of a few short, noisy fragments (a cough, crosstalk, a door
// slam). Any detected speaker whose *total* speaking time across the whole
// recording is below this is almost certainly such an artifact, so we fold
// its segments into the temporally-nearest surviving speaker instead of
// reporting it as a real participant.
const MIN_SPEAKER_TOTAL_SECS: f32 = 3.0;

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

    // Folds phantom speakers (total speaking time < MIN_SPEAKER_TOTAL_SECS)
    // into the surviving speaker whose nearest segment is closest in time,
    // then renumbers the survivors to a contiguous 0..N range. Only used on
    // the auto path — when the count is pinned we trust it as-is.
    fn merge_marginal_speakers(mut raw: Vec<(f32, f32, i32)>) -> Vec<(f32, f32, i32)> {
        use std::collections::HashMap;

        if raw.is_empty() {
            return raw;
        }

        let mut totals: HashMap<i32, f32> = HashMap::new();
        for &(start, end, spk) in &raw {
            *totals.entry(spk).or_insert(0.0) += end - start;
        }

        let survivors: Vec<i32> = totals
            .iter()
            .filter(|(_, &total)| total >= MIN_SPEAKER_TOTAL_SECS)
            .map(|(&spk, _)| spk)
            .collect();

        // If everyone is marginal (e.g. a very short recording), keep the
        // single loudest-present speaker rather than wiping the result.
        let survivors = if survivors.is_empty() {
            match totals.iter().max_by(|a, b| a.1.total_cmp(b.1)) {
                Some((&spk, _)) => vec![spk],
                None => return raw,
            }
        } else {
            survivors
        };

        // Snapshot survivor midpoints before mutating `raw`, so the lookup
        // below doesn't need to borrow it while we're writing to it.
        let survivor_midpoints: Vec<(f32, i32)> = raw
            .iter()
            .filter(|(_, _, s)| survivors.contains(s))
            .map(|&(start, end, spk)| ((start + end) / 2.0, spk))
            .collect();

        for seg in &mut raw {
            if survivors.contains(&seg.2) {
                continue;
            }
            let mid = (seg.0 + seg.1) / 2.0;
            // Reassign to the survivor whose closest segment is nearest in time.
            if let Some(&(_, nearest)) = survivor_midpoints
                .iter()
                .min_by(|a, b| (a.0 - mid).abs().total_cmp(&(b.0 - mid).abs()))
            {
                seg.2 = nearest;
            }
        }

        // Renumber survivors to contiguous 0..N in first-appearance order.
        let mut remap: HashMap<i32, i32> = HashMap::new();
        let mut next = 0;
        for seg in &mut raw {
            let id = *remap.entry(seg.2).or_insert_with(|| {
                let v = next;
                next += 1;
                v
            });
            seg.2 = id;
        }

        raw
    }

    fn diarize_blocking(
        audio_path: &str,
        num_speakers: Option<i32>,
    ) -> Result<Vec<Segment>, ResomerError> {
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
                // When the user tells us the participant count, pin it — this
                // is the single most reliable knob against phantom speakers.
                // Otherwise fall back to auto-detect with a tuned threshold.
                num_clusters: match num_speakers {
                    Some(n) if n > 0 => n,
                    _ => -1,
                },
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

        let mut raw: Vec<(f32, f32, i32)> = result
            .sort_by_start_time()
            .into_iter()
            .map(|s| (s.start, s.end, s.speaker))
            .collect();

        // Only clean up phantom speakers when we let the model guess the
        // count. If the caller pinned it, trust the requested number.
        if num_speakers.is_none() {
            raw = Self::merge_marginal_speakers(raw);
        }

        Ok(raw
            .into_iter()
            .map(|(start, end, speaker)| Segment {
                start,
                end,
                speaker: format!("Speaker-{}", speaker),
            })
            .collect())
    }
}

#[async_trait]
impl crate::domain::DiarizationEngine for SherpaDiarizationEngine {
    async fn diarize(
        &self,
        audio_path: &str,
        num_speakers: Option<i32>,
    ) -> Result<Vec<Segment>, ResomerError> {
        let audio_path = audio_path.to_string();
        tokio::task::spawn_blocking(move || Self::diarize_blocking(&audio_path, num_speakers))
            .await
            .map_err(|e| ResomerError::Diarization(format!("Diarization task panicked: {}", e)))?
    }
}
