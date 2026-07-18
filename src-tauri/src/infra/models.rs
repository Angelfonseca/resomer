use crate::ResomerError;
use std::path::PathBuf;

/// Resolved filesystem paths to the ONNX models used for diarization.
///
/// Models are not bundled in the repo — run `./scripts/setup-models.sh` to
/// download them into `src-tauri/models/` (gitignored).
pub struct DiarizationModelPaths {
    pub segmentation: PathBuf,
    pub embedding: PathBuf,
}

impl DiarizationModelPaths {
    pub fn resolve() -> Result<Self, ResomerError> {
        // CARGO_MANIFEST_DIR is baked in at compile time and always points to
        // `src-tauri/`, regardless of the process's runtime working directory.
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("models");
        let segmentation = base.join("pyannote-segmentation-3.0.onnx");
        let embedding = base.join("speaker-embedding-campplus-en.onnx");

        if !segmentation.exists() || !embedding.exists() {
            return Err(ResomerError::Diarization(format!(
                "Modelos de diarización no encontrados en {}. Ejecuta ./scripts/setup-models.sh",
                base.display()
            )));
        }

        Ok(Self {
            segmentation,
            embedding,
        })
    }
}
