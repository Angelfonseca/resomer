use crate::ResomerError;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Un turno de la conversación de "preguntas sobre la reunión", persistido
/// para que el historial sobreviva al cerrar y reabrir la app.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "user" | "assistant"
    pub content: String,
    pub created_at: String,
}

/// Una conversación del asistente global. A diferencia del chat por reunión
/// (atado a un `meeting_id`), el asistente global soporta varias
/// conversaciones independientes que el usuario crea y renombra a voluntad.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub created_at: String,
}

/// Responde preguntas sobre una reunión usando su transcripción completa
/// (y resumen, si existe) como único contexto — sin retrieval/chunking: para
/// las duraciones típicas de una reunión, el transcript completo cabe sin
/// problema en la ventana de contexto del modelo.
pub struct MeetingChatAssistant {
    api_endpoint: String,
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl MeetingChatAssistant {
    pub fn new(api_endpoint: String, api_key: String, model: String) -> Self {
        Self {
            api_endpoint,
            api_key,
            model,
            client: crate::infra::http_client::build_client(),
        }
    }

    fn system_prompt(transcript: &str, summary: Option<&str>) -> String {
        let mut prompt = format!(
            "Eres un asistente que responde preguntas sobre una reunión, basándote \
             ÚNICAMENTE en la transcripción proporcionada a continuación. Si la \
             respuesta no está en la transcripción, dilo claramente en vez de \
             inventar información. También puedes razonar sobre hipótesis o \
             escenarios que el usuario plantee, siempre dejando claro qué es un \
             hecho de la transcripción y qué es una inferencia o suposición tuya. \
             Responde siempre en español, de forma clara y directa. Usa Markdown \
             cuando ayude a la claridad (negritas para nombres o términos clave, \
             listas con guiones para enumerar puntos, encabezados ## solo si la \
             respuesta tiene varias secciones) — no fuerces estructura en \
             respuestas cortas de una sola frase.\n\n\
             Transcripción de la reunión:\n{}",
            transcript
        );

        if let Some(summary) = summary {
            prompt.push_str(&format!("\n\nResumen ejecutivo ya generado:\n{}", summary));
        }

        prompt
    }

    /// Envía la pregunta junto con el historial previo y el contexto de la
    /// reunión, y devuelve la respuesta del asistente (sin persistirla — eso
    /// lo hace quien llama, vía MeetingRepositoryImpl::save_chat_message).
    pub async fn ask(
        &self,
        transcript: &str,
        summary: Option<&str>,
        history: &[ChatMessage],
        question: &str,
    ) -> Result<String, ResomerError> {
        let mut messages = vec![json!({
            "role": "system",
            "content": Self::system_prompt(transcript, summary),
        })];

        for m in history {
            messages.push(json!({ "role": m.role, "content": m.content }));
        }

        messages.push(json!({ "role": "user", "content": question }));

        let response = self
            .client
            .post(&self.api_endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "messages": messages,
            }))
            .send()
            .await
            .map_err(|e| ResomerError::Chat(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body_text = response.text().await.unwrap_or_default();
            return Err(ResomerError::Chat(format!(
                "API returned status: {} — {}",
                status, body_text
            )));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ResomerError::Chat(format!("Invalid response: {}", e)))?;

        body["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| ResomerError::Chat("Missing content in response".to_string()))
    }
}

/// Una fuente citable para el asistente global: un fragmento de una
/// reunión concreta, recuperado por búsqueda semántica + rerank.
pub struct SourceChunk {
    pub meeting_id: String,
    pub meeting_title: String,
    pub text: String,
}

/// Responde preguntas sobre TODAS las reuniones, a diferencia de
/// `MeetingChatAssistant` que solo conoce una. En vez del transcript
/// completo (inviable con muchas reuniones), recibe los fragmentos más
/// relevantes ya seleccionados por búsqueda semántica + rerank (ver
/// `lib.rs::ask_global_question`), y cita de qué reunión sale cada dato para
/// que el usuario pueda abrirla desde la respuesta.
pub struct GlobalAssistant {
    api_endpoint: String,
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl GlobalAssistant {
    pub fn new(api_endpoint: String, api_key: String, model: String) -> Self {
        Self {
            api_endpoint,
            api_key,
            model,
            client: crate::infra::http_client::build_client(),
        }
    }

    fn system_prompt(sources: &[SourceChunk]) -> String {
        let mut prompt = String::from(
            "Eres un asistente que responde preguntas sobre el historial de reuniones \
             del usuario, basándote ÚNICAMENTE en los fragmentos de transcripción \
             proporcionados a continuación (cada uno etiquetado con la reunión de la \
             que proviene). Si la respuesta no está en estos fragmentos, dilo \
             claramente en vez de inventar información — puede ser que la reunión \
             relevante no esté entre los fragmentos recuperados. Cuando cites un \
             dato, menciona el título de la reunión de la que proviene. Responde \
             siempre en español, de forma clara y directa. Usa Markdown cuando \
             ayude a la claridad (negritas para nombres o términos clave, listas \
             con guiones para enumerar puntos, encabezados ## solo si la respuesta \
             cubre varias reuniones distintas) — no fuerces estructura en \
             respuestas cortas de una sola frase.\n\n\
             Fragmentos recuperados:\n",
        );

        for chunk in sources {
            prompt.push_str(&format!(
                "\n[Reunión: \"{}\"]\n{}\n",
                chunk.meeting_title, chunk.text
            ));
        }

        prompt
    }

    pub async fn ask(
        &self,
        sources: &[SourceChunk],
        history: &[ChatMessage],
        question: &str,
    ) -> Result<String, ResomerError> {
        let mut messages = vec![json!({
            "role": "system",
            "content": Self::system_prompt(sources),
        })];

        for m in history {
            messages.push(json!({ "role": m.role, "content": m.content }));
        }

        messages.push(json!({ "role": "user", "content": question }));

        let response = self
            .client
            .post(&self.api_endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "messages": messages,
            }))
            .send()
            .await
            .map_err(|e| ResomerError::Chat(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body_text = response.text().await.unwrap_or_default();
            return Err(ResomerError::Chat(format!(
                "API returned status: {} — {}",
                status, body_text
            )));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ResomerError::Chat(format!("Invalid response: {}", e)))?;

        body["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| ResomerError::Chat("Missing content in response".to_string()))
    }
}
