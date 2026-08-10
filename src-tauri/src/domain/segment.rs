use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Segment {
    pub start: f32,
    pub end: f32,
    pub speaker: String,
}

impl Segment {
    pub fn duration(&self) -> f32 {
        self.end - self.start
    }
}

/// Una intervención atribuida: un tramo de transcripción (texto) ya asignado
/// a un hablante concreto y con sus tiempos. Es el resultado de cruzar la
/// diarización (quién habló y cuándo) con la transcripción con marcas de
/// tiempo (qué se dijo y cuándo) — la unidad de "quién dijo qué" que ve el
/// usuario. Varias intervenciones consecutivas del mismo hablante se fusionan
/// en una sola durante la alineación.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SpeakerUtterance {
    pub speaker: String,
    pub start: f32,
    pub end: f32,
    pub text: String,
}
