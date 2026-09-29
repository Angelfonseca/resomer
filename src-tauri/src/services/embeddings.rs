use crate::ResomerError;
use serde_json::json;

/// Divide un texto largo en fragmentos de como máximo `max_chars`,
/// cortando en límites de palabra (nunca a mitad de una) para que cada
/// fragmento siga siendo legible como contexto independiente. Sin overlap:
/// a la escala de una transcripción de reunión no aporta suficiente mejora
/// de recall para justificar la complejidad de fragmentos solapados.
pub fn chunk_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if !current.is_empty() && current.len() + 1 + word.len() > max_chars {
            chunks.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Cliente para el endpoint de embeddings del gateway (compatible con el
/// SDK de OpenAI: POST /embeddings con {model, input, encoding_format}).
pub struct EmbeddingClient {
    api_endpoint: String,
    api_key: String,
    client: reqwest::Client,
}

impl EmbeddingClient {
    pub fn new(api_endpoint: String, api_key: String) -> Self {
        Self {
            api_endpoint,
            api_key,
            client: crate::infra::http_client::build_client(),
        }
    }

    pub async fn embed(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, ResomerError> {
        if inputs.is_empty() {
            return Ok(vec![]);
        }

        let response = self
            .client
            .post(&self.api_endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": "qwen3-embedding",
                "input": inputs,
                "encoding_format": "float",
            }))
            .send()
            .await
            .map_err(|e| ResomerError::Embedding(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body_text = response.text().await.unwrap_or_default();
            return Err(ResomerError::Embedding(format!(
                "API returned status: {} — {}",
                status, body_text
            )));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ResomerError::Embedding(format!("Invalid response: {}", e)))?;

        let data = body["data"]
            .as_array()
            .ok_or_else(|| ResomerError::Embedding("Missing data in response".to_string()))?;

        data.iter()
            .map(|entry| {
                entry["embedding"]
                    .as_array()
                    .ok_or_else(|| ResomerError::Embedding("Missing embedding field".to_string()))?
                    .iter()
                    .map(|v| {
                        v.as_f64().map(|f| f as f32).ok_or_else(|| {
                            ResomerError::Embedding("Non-numeric embedding value".to_string())
                        })
                    })
                    .collect::<Result<Vec<f32>, ResomerError>>()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_text_splits_on_word_boundaries() {
        let chunks = chunk_text("uno dos tres cuatro cinco", 11);
        assert_eq!(chunks, vec!["uno dos", "tres cuatro", "cinco"]);
    }

    #[test]
    fn chunk_text_single_chunk_when_fits() {
        assert_eq!(chunk_text("hola mundo", 100), vec!["hola mundo"]);
    }

    #[test]
    fn chunk_text_empty_input() {
        assert!(chunk_text("   ", 10).is_empty());
    }

    #[test]
    fn cosine_similarity_identity_orthogonal_and_zero() {
        assert!((cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        assert!(cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-6);
        assert_eq!(cosine_similarity(&[0.0, 0.0], &[1.0, 1.0]), 0.0);
    }
}
