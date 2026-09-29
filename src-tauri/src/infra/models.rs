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
        let segmentation_name = "pyannote-segmentation-3.0.onnx";
        let embedding_name = "speaker-embedding-campplus-en.onnx";

        // Se prueban varias ubicaciones en orden de prioridad porque
        // `CARGO_MANIFEST_DIR` apunta a la máquina de build y no existe en la
        // app instalada: en un bundle macOS los modelos van en
        // `Contents/Resources/models`.
        let mut dirs: Vec<PathBuf> = Vec::new();
        if let Ok(dir) = std::env::var("RESOMER_MODELS_DIR") {
            dirs.push(PathBuf::from(dir));
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                // Bundle macOS: Contents/MacOS/<exe> → Contents/Resources/models
                dirs.push(parent.join("../Resources/models"));
                dirs.push(parent.join("models"));
            }
        }
        // Desarrollo (`tauri dev`).
        dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("models"));

        for base in &dirs {
            let segmentation = base.join(segmentation_name);
            let embedding = base.join(embedding_name);
            if segmentation.exists() && embedding.exists() {
                return Ok(Self {
                    segmentation,
                    embedding,
                });
            }
        }

        Err(ResomerError::Diarization(format!(
            "Modelos de diarización no encontrados. Buscados en: {}. Ejecuta ./scripts/setup-models.sh",
            dirs.iter()
                .map(|d| d.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }
}
