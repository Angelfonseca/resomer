use crate::ResomerError;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Clone)]
pub struct RerankResult {
    pub index: usize,
    pub relevance_score: f32,
}

#[derive(Deserialize)]
struct RerankResponse {
    results: Vec<RerankResultDto>,
}

#[derive(Deserialize)]
struct RerankResultDto {
    index: usize,
    relevance_score: f32,
}

/// Cliente para el endpoint de rerank del gateway: POST /rerank con
/// {model: "rerank", query, documents} -> {results: [{index, relevance_score}]}.
/// `index` referencia la posición del documento en el array `documents`
/// enviado, no un id propio.
pub struct RerankClient {
    api_endpoint: String,
    api_key: String,
    client: reqwest::Client,
}

impl RerankClient {
    pub fn new(api_endpoint: String, api_key: String) -> Self {
        Self {
            api_endpoint,
            api_key,
            client: crate::infra::http_client::build_client(),
        }
    }

    /// Devuelve los resultados ordenados de mayor a menor relevancia — no
    /// asumimos que el gateway ya los entregue ordenados.
    pub async fn rerank(
        &self,
        query: &str,
        documents: &[String],
    ) -> Result<Vec<RerankResult>, ResomerError> {
        if documents.is_empty() {
            return Ok(vec![]);
        }

        let response = self
            .client
            .post(&self.api_endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": "rerank",
                "query": query,
                "documents": documents,
            }))
            .send()
            .await
            .map_err(|e| ResomerError::Rerank(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body_text = response.text().await.unwrap_or_default();
            return Err(ResomerError::Rerank(format!(
                "API returned status: {} — {}",
                status, body_text
            )));
        }

        let parsed: RerankResponse = response
            .json()
            .await
            .map_err(|e| ResomerError::Rerank(format!("Invalid response: {}", e)))?;

        let mut results: Vec<RerankResult> = parsed
            .results
            .into_iter()
            .map(|r| RerankResult {
                index: r.index,
                relevance_score: r.relevance_score,
            })
            .collect();

        results.sort_by(|a, b| b.relevance_score.total_cmp(&a.relevance_score));
        Ok(results)
    }
}
