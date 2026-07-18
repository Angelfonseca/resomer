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

/// Duración máxima por fragmento subido al endpoint de transcripción.
///
/// La documentación del gateway (api.nan.builders) es explícita: Whisper
/// corre en CPU a ~1x tiempo real, y para audios de más de 2 minutos "el
/// proxy puede devolver un 524 (timeout) antes de que termine la
/// transcripción". El código anterior troceaba a 300s (5 min), muy por
/// encima de ese límite documentado — probablemente la causa real de los
/// 500/"Internal Server Error" reportados, no solo un fallo transitorio del
/// backend. 100s deja margen bajo el límite de 120s.
///
/// A mono 16 kHz 16-bit, un fragmento de 100s pesa ~3.2 MB, muy por debajo
/// del límite de tamaño documentado (25 MB por request).
const MAX_CHUNK_DURATION_SECS: f32 = 100.0;

/// Umbral de RMS (escala i16, máximo 32767) bajo el cual un fragmento se
/// considera silencio digital puro y se salta sin llamar a la API.
///
/// Deliberadamente muy bajo (solo atrapa audio literalmente vacío/plano).
/// Confirmado en producción (2026-07-18): un WAV de silencio digital puro
/// (RMS 0) hace que el backend de Whisper de este gateway devuelva un 500
/// "litellm.InternalServerError ... Model Group=whisper" de forma
/// reproducible. PERO se descartó usar un umbral más alto para detectar
/// "casi silencio" en general: al medir un fragmento real que también
/// disparaba el mismo 500 (la cola de una grabación, sin voz) contra otro
/// fragmento de la MISMA grabación que sí transcribe bien, ambos resultaron
/// tener estadísticas de energía por ventana casi idénticas (grabación de
/// mic distante/bajo volumen en general) — no hay forma fiable de
/// distinguir "silencio real" de "voz grabada bajito" con un umbral de
/// amplitud aquí. La defensa real contra ese caso es el manejo de fallo por
/// fragmento en `transcribe()` (ver más abajo), no este chequeo.
const SILENCE_RMS_THRESHOLD: f64 = 5.0;

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

    /// True si el fragmento es esencialmente silencio (ver
    /// SILENCE_RMS_THRESHOLD) y por tanto no debe enviarse a Whisper.
    fn is_effectively_silent(samples: &[i16]) -> bool {
        if samples.is_empty() {
            return true;
        }
        let sum_sq: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
        let rms = (sum_sq / samples.len() as f64).sqrt();
        rms < SILENCE_RMS_THRESHOLD
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

    // Divide audio (ya normalizado a mono 16 kHz) en chunks de máximo
    // max_duration_secs, retornando índices de muestra.
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
        let (spec, samples, chunks) = Self::chunk_audio_file(audio_path, MAX_CHUNK_DURATION_SECS)?;
        let total_chunks = chunks.len();

        let mut all_transcripts = vec![];
        let mut failed_chunks = 0usize;
        let mut last_error: Option<ResomerError> = None;

        for (start, end) in chunks {
            let chunk_samples = &samples[start..end];

            // Silencio digital puro: no aporta texto y confirmadamente hace
            // que este backend devuelva un 500 (ver SILENCE_RMS_THRESHOLD).
            if Self::is_effectively_silent(chunk_samples) {
                continue;
            }

            let wav_bytes = Self::encode_chunk_wav(spec, chunk_samples)?;

            // Un fragmento individual puede fallar por razones ajenas a
            // nuestro control (p. ej. este backend a veces devuelve 500 en
            // tramos sin voz detectable, incluso cuando no son silencio
            // digital puro — ver nota en SILENCE_RMS_THRESHOLD). En vez de
            // abortar toda la transcripción de la reunión por un solo
            // fragmento problemático, lo saltamos y seguimos con el resto.
            match self.transcribe_chunk(wav_bytes).await {
                Ok(transcript) => all_transcripts.push(transcript),
                Err(e) => {
                    failed_chunks += 1;
                    last_error = Some(e);
                }
            }
        }

        if all_transcripts.is_empty() && failed_chunks > 0 {
            // Ningún fragmento se transcribió: sí es un fallo real que hay
            // que reportar, con el último error de fondo para diagnóstico.
            return Err(ResomerError::Transcription(format!(
                "Ningún fragmento de audio pudo transcribirse ({}/{} fallaron). Último error: {}",
                failed_chunks,
                total_chunks,
                last_error
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "desconocido".to_string())
            )));
        }

        let mut result = all_transcripts.join(" ");
        if failed_chunks > 0 {
            result.push_str(&format!(
                "\n\n[Nota: {} de {} fragmentos de audio no pudieron transcribirse y se omitieron.]",
                failed_chunks, total_chunks
            ));
        }

        Ok(result)
    }
}

#[cfg(test)]
mod manual_verification {
    use super::*;
    use crate::domain::Transcriber;

    // Test manual, ignorado por defecto: llama al gateway real con una
    // grabación real que antes hacía fallar toda la transcripción por un
    // fragmento de cola silenciosa. Requiere red y una API key válida en el
    // Keychain. Ejecutar con:
    //   cargo test --lib manual_verification -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn real_recording_with_silent_tail_does_not_abort() {
        let api_key = crate::infra::keychain::KeychainManager::get_api_key()
            .expect("keychain read failed")
            .expect("no API key saved");

        let transcriber = CloudTranscriber::new(
            "https://api.nan.builders/v1/audio/transcriptions".to_string(),
            api_key,
            "whisper".to_string(),
        );

        let path = std::env::var("TEST_AUDIO_PATH")
            .expect("set TEST_AUDIO_PATH to a real recording's .wav path");

        let result = transcriber.transcribe(&path).await;
        match &result {
            Ok(text) => println!("OK ({} chars): {}", text.len(), text),
            Err(e) => println!("ERROR: {}", e),
        }
        assert!(result.is_ok(), "la transcripción no debería abortar por completo");
    }
}
