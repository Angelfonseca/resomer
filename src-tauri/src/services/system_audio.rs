use crate::domain::AudioRecorder;
use crate::ResomerError;
use async_trait::async_trait;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;
use tauri::Emitter;

/// Ruta al binario del helper de Swift, inyectada por build.rs en tiempo de
/// compilación. El helper usa ScreenCaptureKit para capturar el audio del
/// sistema (audio interno) de forma nativa, sin depender de apps externas.
const HELPER_PATH: Option<&str> = option_env!("RESOMER_AUDIO_HELPER");

/// Evento Tauri emitido con el nivel de audio en vivo (0.0–1.0) mientras se
/// captura audio del sistema, para que la UI pueda visualizarlo.
const LEVEL_EVENT: &str = "system-audio-level";

/// Graba el audio del sistema (audio interno) lanzando el helper de Swift
/// basado en ScreenCaptureKit y controlándolo como proceso hijo. Se detiene
/// enviando SIGTERM, con lo que el helper finaliza el WAV de forma segura.
///
/// Si `capture_mic` es `true`, el helper también captura el micrófono y lo
/// mezcla en la misma pista (modo "ambos").
pub struct SystemAudioRecorder {
    child: Mutex<Option<Child>>,
    capture_mic: bool,
    app: tauri::AppHandle,
}

impl SystemAudioRecorder {
    /// Crea un grabador de solo audio del sistema.
    pub fn new(app: tauri::AppHandle) -> Result<Self, ResomerError> {
        Self::with_options(app, false)
    }

    /// Crea un grabador con opción de mezclar el micrófono.
    pub fn with_options(app: tauri::AppHandle, capture_mic: bool) -> Result<Self, ResomerError> {
        Ok(Self {
            child: Mutex::new(None),
            capture_mic,
            app,
        })
    }

    /// Traduce el stderr del helper a un mensaje accionable para el usuario.
    fn describe_startup_error(stderr: &str) -> String {
        let s = stderr.to_lowercase();
        if s.contains("tcc") || s.contains("-3801") || s.contains("declin") || s.contains("rechaz")
        {
            "Permiso de Grabación de pantalla denegado. Actívalo en Ajustes del Sistema › \
             Privacidad y seguridad › Grabación de pantalla, marca Resomer y vuelve a intentarlo. \
             (macOS usa este permiso para capturar el audio del sistema.)"
                .to_string()
        } else if stderr.trim().is_empty() {
            "El helper de captura de audio del sistema terminó inesperadamente.".to_string()
        } else {
            format!(
                "No se pudo iniciar la captura de audio del sistema: {}",
                stderr.trim()
            )
        }
    }

    fn helper_path() -> Result<PathBuf, ResomerError> {
        // Prioridad 1: el helper empaquetado junto al ejecutable (app instalada).
        // Es importante para TCC: macOS asocia el permiso de Grabación de
        // pantalla al binario dentro del bundle, no a uno externo.
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let candidate = dir.join("resomer-audio-helper");
                if candidate.exists() {
                    return Ok(candidate);
                }
            }
        }

        // Prioridad 2: la ruta baked por build.rs (OUT_DIR) — útil en `tauri dev`,
        // donde no hay bundle y el ejecutable vive en target/debug.
        if let Some(p) = HELPER_PATH {
            let path = PathBuf::from(p);
            if path.exists() {
                return Ok(path);
            }
        }

        Err(ResomerError::RecordingError(
            "No se encontró el helper de captura de audio del sistema".to_string(),
        ))
    }

    /// Envía SIGTERM al proceso hijo para que finalice el WAV limpiamente.
    fn terminate_gracefully(child: &mut Child) -> Result<(), ResomerError> {
        let pid = child.id() as i32;

        // SIGTERM: el helper lo captura, detiene la captura y cierra el WAV.
        unsafe {
            libc::kill(pid, libc::SIGTERM);
        }

        // Esperar a que finalice (con timeout para no bloquear indefinidamente).
        for _ in 0..50 {
            match child.try_wait() {
                Ok(Some(_)) => return Ok(()),
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
                Err(e) => {
                    return Err(ResomerError::RecordingError(format!(
                        "Error esperando al helper: {}",
                        e
                    )))
                }
            }
        }

        // Si no terminó en ~5s, forzar.
        let _ = child.kill();
        let _ = child.wait();
        Ok(())
    }
}

#[async_trait]
impl AudioRecorder for SystemAudioRecorder {
    async fn start_recording(&self, output_path: &str) -> Result<(), ResomerError> {
        // Crear el directorio de salida si hace falta.
        if let Some(parent) = Path::new(output_path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ResomerError::RecordingError(format!("Dir creation failed: {}", e))
                })?;
            }
        }

        let helper = Self::helper_path()?;

        let mut command = Command::new(&helper);
        command.arg(output_path);
        if self.capture_mic {
            command.arg("--mic");
        }

        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                ResomerError::RecordingError(format!("No se pudo lanzar el helper de audio: {}", e))
            })?;

        // Detectar fallo temprano (p. ej. permiso de Grabación de pantalla
        // denegado): si el helper sale en el primer ~1.5s, leer stderr y
        // devolver un mensaje claro en vez de fingir que está grabando.
        for _ in 0..15 {
            match child.try_wait() {
                Ok(Some(_status)) => {
                    let mut err = String::new();
                    if let Some(mut se) = child.stderr.take() {
                        let _ = se.read_to_string(&mut err);
                    }
                    return Err(ResomerError::RecordingError(Self::describe_startup_error(&err)));
                }
                Ok(None) => tokio::time::sleep(Duration::from_millis(100)).await,
                Err(e) => {
                    return Err(ResomerError::RecordingError(format!(
                        "Error monitorizando el helper de audio: {}",
                        e
                    )))
                }
            }
        }

        // El helper sigue vivo: leer su stdout en un hilo aparte y reenviar
        // cada línea "LVL <0..1>" como evento Tauri para el medidor en vivo.
        // Es imprescindible consumir el stdout del hijo: si nadie lee y el
        // pipe se llena, el helper se bloquearía al escribir.
        if let Some(stdout) = child.stdout.take() {
            let app = self.app.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    let Ok(line) = line else { break };
                    if let Some(rest) = line.strip_prefix("LVL ") {
                        if let Ok(level) = rest.trim().parse::<f32>() {
                            let _ = app.emit(LEVEL_EVENT, level);
                        }
                    }
                }
            });
        }

        *self.child.lock().unwrap() = Some(child);
        Ok(())
    }

    async fn stop_recording(&self) -> Result<(), ResomerError> {
        let child = self.child.lock().unwrap().take();
        if let Some(mut child) = child {
            Self::terminate_gracefully(&mut child)?;
        }
        Ok(())
    }

    async fn pause_recording(&self) -> Result<(), ResomerError> {
        // ScreenCaptureKit no expone pausa nativa; detenemos la captura.
        // (El flujo de la app trata pausa como detener temporalmente.)
        let child = self.child.lock().unwrap().take();
        if let Some(mut child) = child {
            Self::terminate_gracefully(&mut child)?;
        }
        Ok(())
    }
}
