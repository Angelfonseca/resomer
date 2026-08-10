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

    // Prompt en español para resumen. `instructions` son indicaciones del
    // usuario sobre qué espera del resumen (p. ej. "enfócate en lo técnico",
    // "más corto", "no menciones nombres") — se priorizan sobre la estructura
    // por defecto cuando entran en conflicto.
    fn create_summary_prompt(transcript: &str, instructions: Option<&str>) -> String {
        let instructions_block = match instructions {
            Some(i) if !i.trim().is_empty() => format!(
                "\nInstrucciones del usuario sobre este resumen (tenlas en cuenta y \
                 priorízalas por encima de la estructura por defecto si entran en \
                 conflicto):\n{}\n",
                i.trim()
            ),
            _ => String::new(),
        };

        format!(
            r#"Analiza el siguiente transcript de una reunión y redacta un resumen ejecutivo extenso y detallado en español usando Markdown. No te limites a un esquema mínimo: desarrolla cada sección con la profundidad suficiente para que alguien que no asistió entienda por completo qué pasó, por qué, y qué sigue.
{}
Estructura el resumen así:

## Resumen Ejecutivo
[3-4 párrafos que expliquen el propósito de la reunión, el contexto, los puntos de discusión más importantes y las conclusiones generales. Sé narrativo y completo, no telegráfico.]

## Temas Principales
Para cada tema relevante, usa un subtítulo ### con el nombre del tema seguido de 1-2 párrafos que expliquen qué se discutió, qué posturas surgieron y en qué se concluyó.

## Decisiones Tomadas
- [Cada decisión con una frase de contexto que explique por qué se tomó]

## Próximos Pasos
- [ ] [Acción concreta] — Responsable: [Persona] — Plazo: [si se mencionó]

## Puntos Abiertos / Pendientes de Definir
- [Temas que quedaron sin resolver o que requieren seguimiento]

## Notas Adicionales
[Cualquier detalle importante: cifras, nombres, referencias, riesgos o menciones relevantes]

Recuerda:
- Usa Markdown válido (encabezados ##/###, listas con guiones, casillas [ ] para acciones)
- Sé exhaustivo pero claro; prioriza la utilidad sobre la brevedad
- Mantén todo el contenido en español
- Si una sección no aplica porque no hubo contenido, omítela en vez de inventar

Transcript:
{}

Resumen:"#,
            instructions_block, transcript
        )
    }

    /// Resumen con instrucciones opcionales del usuario sobre qué espera del
    /// resultado (p. ej. "enfócate en las decisiones técnicas", "más breve").
    /// Es el método que usan tanto el resumen inicial del pipeline como
    /// "Regenerar" — la única diferencia entre ambos es si hay `instructions`.
    pub async fn summarize_with_instructions(
        &self,
        text: &str,
        instructions: Option<&str>,
    ) -> Result<String, ResomerError> {
        let chunks = Self::chunk_text(text, 2000); // Máx 2000 tokens por chunk

        if chunks.len() > 1 {
            // Para textos muy largos, hacer un resumen de dos fases
            let mut chunk_summaries = vec![];

            for chunk in chunks {
                let prompt = Self::create_summary_prompt(&chunk, instructions);
                let summary = self.call_llm(&prompt).await?;
                chunk_summaries.push(summary);
            }

            // Combinar summaries y hacer resumen final
            let combined = chunk_summaries.join("\n\n");
            let final_prompt = Self::create_summary_prompt(&combined, instructions);
            self.call_llm(&final_prompt).await
        } else {
            // Texto corto, resumen directo
            let prompt = Self::create_summary_prompt(text, instructions);
            self.call_llm(&prompt).await
        }
    }

    /// Genera un título corto y descriptivo para la reunión a partir de su
    /// resumen (o transcript). Una sola llamada barata: el frontend lo dispara
    /// tras el resumen y lo deja editable.
    pub async fn generate_title(&self, text: &str) -> Result<String, ResomerError> {
        // Solo el inicio: para titular basta el arranque de la reunión y evita
        // mandar transcripts enormes por una frase.
        let excerpt: String = text.chars().take(4000).collect();
        let prompt = format!(
            r#"Genera un título corto y descriptivo en español para esta reunión, de 3 a 8 palabras. Debe capturar el tema central. Responde ÚNICAMENTE con el título, sin comillas, sin punto final, sin prefijos como "Título:".

Contenido de la reunión:
{}

Título:"#,
            excerpt
        );
        let title = self.call_llm(&prompt).await?;
        // Saneado: una sola línea, sin comillas ni longitud excesiva.
        let title = title
            .trim()
            .lines()
            .next()
            .unwrap_or("")
            .trim_matches(|c| c == '"' || c == '\'' || c == '.')
            .trim()
            .to_string();
        Ok(if title.is_empty() {
            "Reunión".to_string()
        } else {
            title.chars().take(80).collect()
        })
    }
}

#[async_trait]
impl crate::domain::Summarizer for LlmSummarizer {
    async fn summarize(&self, text: &str) -> Result<String, ResomerError> {
        self.summarize_with_instructions(text, None).await
    }
}
