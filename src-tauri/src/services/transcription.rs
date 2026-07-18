use crate::ResomerError;
use async_trait::async_trait;
use hound::{WavReader, WavSpec, WavWriter};
use serde::{Deserialize, Serialize};
use sherpa_onnx::LinearResampler;
use std::io::Cursor;
use std::path::Path;

/// Whisper se entrena y sirve mejor con audio mono a 16 kHz; muchos backends
/// (incluidos gateways self-hosted) fallan con audio estéreo o a otras
/// frecuencias, a veces con un 500 genérico en vez de un error claro. Bajamos
/// siempre a este formato antes de subir, independientemente de cómo se haya
/// grabado (mic mono, o sistema/"ambos" en estéreo a 44.1/48 kHz).
const WHISPER_SAMPLE_RATE: i32 = 16_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub start: f32,
    pub end: f32,
    pub text: String,
}

pub struct CloudTranscriber {
    api_endpoint: String,
    api_key: String,
    model: String,
    client: reqwest::Client,
}

type ChunkedAudio = (WavSpec, Vec<i16>, Vec<(usize, usize)>);

impl CloudTranscriber {
    pub fn new(api_endpoint: String, api_key: String, model: String) -> Self {
        Self {
            api_endpoint,
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }

    // Downmixea intercalado i16 a mono f32 en [-1.0, 1.0]. (Misma lógica que
    // SherpaDiarizationEngine::to_mono_f32, duplicada para no acoplar los
    // dos servicios a través de una dependencia cruzada innecesaria.)
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

    fn f32_to_i16(samples: &[f32]) -> Vec<i16> {
        samples
            .iter()
            .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
            .collect()
    }

    /// Lee el WAV y lo normaliza a mono 16 kHz antes de trocearlo, sin
    /// importar cómo se haya grabado originalmente (mono, estéreo, 44.1/48 kHz).
    fn load_and_normalize(audio_path: &str) -> Result<(WavSpec, Vec<i16>), ResomerError> {
        let path = Path::new(audio_path);
        let mut reader = WavReader::open(path)
            .map_err(|e| ResomerError::Transcription(format!("Failed to open WAV: {}", e)))?;

        let spec = reader.spec();
        let samples: Vec<i16> = reader
            .samples::<i16>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ResomerError::Transcription(format!("Failed to read samples: {}", e)))?;

        if samples.is_empty() {
            let mono_spec = WavSpec {
                channels: 1,
                sample_rate: WHISPER_SAMPLE_RATE as u32,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };
            return Ok((mono_spec, vec![]));
        }

        let mono = Self::to_mono_f32(&samples, spec.channels);

        let resampled = if spec.sample_rate as i32 != WHISPER_SAMPLE_RATE {
            let resampler = LinearResampler::create(spec.sample_rate as i32, WHISPER_SAMPLE_RATE)
                .ok_or_else(|| {
                    ResomerError::Transcription("No se pudo inicializar el resampler".to_string())
                })?;
            resampler.resample(&mono, true)
        } else {
            mono
        };

        let mono_spec = WavSpec {
            channels: 1,
            sample_rate: WHISPER_SAMPLE_RATE as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        Ok((mono_spec, Self::f32_to_i16(&resampled)))
    }

    // Divide audio (ya normalizado a mono 16 kHz) en chunks ≤ 300 segundos,
    // retornando índices de muestra.
    fn chunk_audio_file(
        audio_path: &str,
        max_duration_secs: f32,
    ) -> Result<ChunkedAudio, ResomerError> {
        let (spec, samples) = Self::load_and_normalize(audio_path)?;

        let sample_rate = spec.sample_rate as f32;
        let samples_per_chunk = (max_duration_secs * sample_rate) as usize;

        let total_samples = samples.len();
        let mut chunks = vec![];
        let mut start = 0;

        while start < total_samples {
            let end = (start + samples_per_chunk).min(total_samples);
            chunks.push((start, end));
            start = end;
        }

        Ok((spec, samples, chunks))
    }

    fn encode_chunk_wav(spec: WavSpec, samples: &[i16]) -> Result<Vec<u8>, ResomerError> {
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut writer = WavWriter::new(&mut buffer, spec)
                .map_err(|e| ResomerError::Transcription(format!("WAV encode failed: {}", e)))?;
            for &sample in samples {
                writer
                    .write_sample(sample)
                    .map_err(|e| ResomerError::Transcription(format!("Write failed: {}", e)))?;
            }
            writer
                .finalize()
                .map_err(|e| ResomerError::Transcription(format!("Finalize failed: {}", e)))?;
        }
        Ok(buffer.into_inner())
    }

    async fn transcribe_chunk(&self, wav_bytes: Vec<u8>) -> Result<String, ResomerError> {
        let part = reqwest::multipart::Part::bytes(wav_bytes)
            .file_name("chunk.wav")
            .mime_str("audio/wav")
            .map_err(|e| ResomerError::Transcription(format!("Multipart error: {}", e)))?;

        let form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("model", self.model.clone());

        let response = self
            .client
            .post(&self.api_endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| ResomerError::Transcription(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body_text = response.text().await.unwrap_or_default();
            return Err(ResomerError::Transcription(format!(
                "API returned status: {} — {}",
                status, body_text
            )));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ResomerError::Transcription(format!("Invalid response: {}", e)))?;

        body["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| ResomerError::Transcription("Missing text in response".to_string()))
    }
}

#[async_trait]
impl crate::domain::Transcriber for CloudTranscriber {
    async fn transcribe(&self, audio_path: &str) -> Result<String, ResomerError> {
        let (spec, samples, chunks) = Self::chunk_audio_file(audio_path, 300.0)?;

        let mut all_transcripts = vec![];

        for (start, end) in chunks {
            let wav_bytes = Self::encode_chunk_wav(spec, &samples[start..end])?;
            let transcript = self.transcribe_chunk(wav_bytes).await?;
            all_transcripts.push(transcript);
        }

        Ok(all_transcripts.join(" "))
    }
}
