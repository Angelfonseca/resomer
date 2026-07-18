use crate::ResomerError;
use async_trait::async_trait;
use serde_json::json;

pub struct LlmSummarizer {
    api_endpoint: String,
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl LlmSummarizer {
    pub fn new(api_endpoint: String, api_key: String, model: String) -> Self {
        Self {
            api_endpoint,
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }

    async fn call_llm(&self, prompt: &str) -> Result<String, ResomerError> {
        let response = self
            .client
            .post(&self.api_endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "messages": [{"role": "user", "content": prompt}],
            }))
            .send()
            .await
            .map_err(|e| ResomerError::Summarization(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body_text = response.text().await.unwrap_or_default();
            return Err(ResomerError::Summarization(format!(
                "API returned status: {} — {}",
                status, body_text
            )));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ResomerError::Summarization(format!("Invalid response: {}", e)))?;

        body["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| ResomerError::Summarization("Missing content in response".to_string()))
    }

    // Divide texto largo en chunks para resumen
    fn chunk_text(text: &str, max_tokens: usize) -> Vec<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let approx_chars_per_token = 4; // Heurística aproximada
        let max_chars = max_tokens * approx_chars_per_token;

        let mut chunks = vec![];
        let mut current_chunk = String::new();

        for word in words {
            if current_chunk.len() + word.len() + 1 > max_chars && !current_chunk.is_empty() {
                chunks.push(current_chunk.clone());
                current_chunk.clear();
            }
            current_chunk.push_str(word);
            current_chunk.push(' ');
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        chunks
    }

    // Prompt en español para resumen
    fn create_summary_prompt(transcript: &str) -> String {
        format!(
            r#"Analiza el siguiente transcript de una reunión y proporciona un resumen ejecutivo en español usando Markdown.

El resumen debe estructurarse así:

## Resumen Ejecutivo
[1-2 párrafos resumiendo el propósito y conclusión de la reunión]

## Temas Principales
- [Tema 1]
- [Tema 2]
- [Tema 3]
- [etc...]

## Decisiones Tomadas
- [Decisión 1]
- [Decisión 2]
- [etc...]

## Próximos Pasos
- [ ] [Acción 1] - Responsable: [Persona]
- [ ] [Acción 2] - Responsable: [Persona]
- [etc...]

## Notas
[Cualquier información adicional importante]

Recuerda:
- Usa Markdown válido con encabezados # y ##
- Usa listas con guiones (-)
- Usa casillas de verificación [ ] para acciones
- Sé conciso y claro
- Mantén el contenido en español

Transcript:
{}

Resumen:"#,
            transcript
        )
    }
}

#[async_trait]
impl crate::domain::Summarizer for LlmSummarizer {
    async fn summarize(&self, text: &str) -> Result<String, ResomerError> {
        // Si el texto es muy largo, resumir en chunks y combinar
        let chunks = Self::chunk_text(text, 2000); // Máx 2000 tokens por chunk

        if chunks.len() > 1 {
            // Para textos muy largos, hacer un resumen de dos fases
            let mut chunk_summaries = vec![];

            for chunk in chunks {
                let prompt = Self::create_summary_prompt(&chunk);
                let summary = self.call_llm(&prompt).await?;
                chunk_summaries.push(summary);
            }

            // Combinar summaries y hacer resumen final
            let combined = chunk_summaries.join("\n\n");
            let final_prompt = Self::create_summary_prompt(&combined);
            self.call_llm(&final_prompt).await
        } else {
            // Texto corto, resumen directo
            let prompt = Self::create_summary_prompt(text);
            self.call_llm(&prompt).await
        }
    }
}
