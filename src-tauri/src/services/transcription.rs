use crate::ResomerError;
use async_trait::async_trait;
use hound::{WavReader, WavSpec, WavWriter};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::Path;

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

    // Divide audio en chunks ≤ 300 segundos, retornando índices de muestra
    fn chunk_audio_file(
        audio_path: &str,
        max_duration_secs: f32,
    ) -> Result<ChunkedAudio, ResomerError> {
        let path = Path::new(audio_path);
        let mut reader = WavReader::open(path)
            .map_err(|e| ResomerError::Transcription(format!("Failed to open WAV: {}", e)))?;

        let spec = reader.spec();
        let sample_rate = spec.sample_rate as f32;
        let samples_per_chunk = (max_duration_secs * sample_rate) as usize;

        let samples: Vec<i16> = reader
            .samples::<i16>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ResomerError::Transcription(format!("Failed to read samples: {}", e)))?;

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
