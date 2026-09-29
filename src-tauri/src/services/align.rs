use crate::domain::{Segment, SpeakerUtterance};
use crate::services::transcription::TranscriptSegment;

/// Cruza la diarización (quién habló y cuándo) con la transcripción con marcas
/// de tiempo (qué se dijo y cuándo) para producir la vista de "quién dijo qué".
///
/// Estrategia: a cada segmento de texto se le asigna el hablante cuyo intervalo
/// de diarización se solapa **más** en el tiempo con él. Si un segmento de
/// texto no solapa con ninguno (huecos de la diarización, o silencios que
/// Whisper igual transcribió), se le asigna el hablante del segmento de
/// diarización más cercano por punto medio. Finalmente, intervenciones
/// consecutivas del mismo hablante se fusionan en una sola para que la lectura
/// sea natural en vez de una lista fragmentada frase a frase.
pub fn attribute_speakers(
    diarization: &[Segment],
    transcript: &[TranscriptSegment],
) -> Vec<SpeakerUtterance> {
    if transcript.is_empty() {
        return vec![];
    }

    // Etiqueta que se usa cuando no hubo diarización en absoluto (p. ej. una
    // grabación demasiado corta o de un único hablante que el motor no
    // segmentó). Todo el texto queda bajo un mismo hablante.
    const FALLBACK_SPEAKER: &str = "Speaker-0";

    let mut merged: Vec<SpeakerUtterance> = Vec::new();

    for seg in transcript {
        let speaker = assign_speaker(diarization, seg).unwrap_or(FALLBACK_SPEAKER);

        // Fusiona con la intervención anterior si es el mismo hablante, para no
        // trocear el texto en decenas de líneas de una frase cada una.
        match merged.last_mut() {
            Some(last) if last.speaker == speaker => {
                last.end = seg.end.max(last.end);
                last.text.push(' ');
                last.text.push_str(seg.text.trim());
            }
            _ => merged.push(SpeakerUtterance {
                speaker: speaker.to_string(),
                start: seg.start,
                end: seg.end,
                text: seg.text.trim().to_string(),
            }),
        }
    }

    merged
}

/// Devuelve el hablante asignado a un segmento de texto: el de mayor
/// solapamiento temporal, o el más cercano por punto medio si no solapa con
/// ninguno. `None` solo si la diarización está vacía.
fn assign_speaker<'a>(diarization: &'a [Segment], seg: &TranscriptSegment) -> Option<&'a str> {
    if diarization.is_empty() {
        return None;
    }

    // 1) Máximo solapamiento.
    let mut best: Option<(f32, &str)> = None;
    for d in diarization {
        let overlap = (seg.end.min(d.end) - seg.start.max(d.start)).max(0.0);
        if overlap > 0.0 && best.map(|(o, _)| overlap > o).unwrap_or(true) {
            best = Some((overlap, d.speaker.as_str()));
        }
    }
    if let Some((_, speaker)) = best {
        return Some(speaker);
    }

    // 2) Sin solapamiento: el hablante del segmento más cercano por punto medio.
    let mid = (seg.start + seg.end) / 2.0;
    diarization
        .iter()
        .min_by(|a, b| {
            let da = ((a.start + a.end) / 2.0 - mid).abs();
            let db = ((b.start + b.end) / 2.0 - mid).abs();
            da.total_cmp(&db)
        })
        .map(|d| d.speaker.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(start: f32, end: f32, text: &str) -> TranscriptSegment {
        TranscriptSegment {
            start,
            end,
            text: text.to_string(),
        }
    }

    fn seg(start: f32, end: f32, speaker: &str) -> Segment {
        Segment {
            start,
            end,
            speaker: speaker.to_string(),
        }
    }

    #[test]
    fn assigns_by_overlap_and_merges_consecutive() {
        let diarization = vec![seg(0.0, 5.0, "Speaker-0"), seg(5.0, 10.0, "Speaker-1")];
        let transcript = vec![
            ts(0.0, 2.0, "Hola"),
            ts(2.0, 4.5, "qué tal"),
            ts(5.5, 8.0, "Todo bien"),
        ];

        let out = attribute_speakers(&diarization, &transcript);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].speaker, "Speaker-0");
        assert_eq!(out[0].text, "Hola qué tal");
        assert_eq!(out[0].start, 0.0);
        assert_eq!(out[0].end, 4.5);
        assert_eq!(out[1].speaker, "Speaker-1");
        assert_eq!(out[1].text, "Todo bien");
    }

    #[test]
    fn no_overlap_falls_back_to_nearest() {
        let diarization = vec![seg(0.0, 3.0, "Speaker-0"), seg(10.0, 13.0, "Speaker-1")];
        // Segmento en un hueco (4-5s): más cercano a Speaker-0.
        let transcript = vec![ts(4.0, 5.0, "puente")];
        let out = attribute_speakers(&diarization, &transcript);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].speaker, "Speaker-0");
    }

    #[test]
    fn empty_diarization_uses_single_fallback_speaker() {
        let transcript = vec![ts(0.0, 2.0, "uno"), ts(2.0, 4.0, "dos")];
        let out = attribute_speakers(&[], &transcript);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].speaker, "Speaker-0");
        assert_eq!(out[0].text, "uno dos");
    }

    #[test]
    fn empty_transcript_yields_nothing() {
        let diarization = vec![seg(0.0, 5.0, "Speaker-0")];
        assert!(attribute_speakers(&diarization, &[]).is_empty());
    }
}
