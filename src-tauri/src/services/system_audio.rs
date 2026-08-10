use crate::domain::AudioRecorder;
use crate::ResomerError;
use async_trait::async_trait;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::Emitter;

/// Ruta al binario del helper de Swift, inyectada por build.rs en tiempo de
/// compilación. El helper usa ScreenCaptureKit para capturar el audio del
/// sistema (audio interno) de forma nativa, sin depender de apps externas.
const HELPER_PATH: Option<&str> = option_env!("RESOMER_AUDIO_HELPER");

/// Evento Tauri emitido con el nivel de audio en vivo (0.0–1.0) de sistema y
/// micrófono por separado mientras se captura audio del sistema, para que la
/// UI (y el ícono de la barra de estado) puedan visualizar ambas señales.
const LEVEL_EVENT: &str = "system-audio-level";

/// Payload del evento de nivel: pico de sistema y de micrófono medidos por
/// separado antes de mezclarlos. `mic` es 0.0 cuando no se captura micrófono.
#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct AudioLevels {
    pub system: f32,
    pub mic: f32,
}

/// Evento Tauri emitido si el helper muere a mitad de grabación (no por un
/// stop/pause pedido desde la app), para que la UI lo muestre en vez de
/// seguir mostrando "grabando" indefinidamente.
const CRASH_EVENT: &str = "system-audio-crashed";

/// Cuánto esperar a que el helper confirme el arranque real de la captura
/// (línea "READY") antes de darlo por fallido. SCShareableContent (el check
/// de permiso de Grabación de pantalla) puede tardar más de un segundo.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);

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
    // Marcado antes de pedirle al helper que termine (stop/pause), para que
    // el hilo lector de stdout distinga "lo detuvimos nosotros" de "el
    // helper murió solo" cuando ve el EOF.
    stopping: Arc<AtomicBool>,
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
            stopping: Arc::new(AtomicBool::new(false)),
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

        self.stopping.store(false, Ordering::SeqCst);

        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                ResomerError::RecordingError(format!("No se pudo lanzar el helper de audio: {}", e))
            })?;

        let stdout = child.stdout.take().expect("stdout piped");

        // Hilo lector: consume stdout durante toda la vida del proceso. Antes
        // de la primera línea "READY" (arranque real de la captura), reenvía
        // esa señal por el canal `ready_tx`. Después, reenvía cada línea
        // "LVL <0..1>" como evento Tauri para el medidor en vivo. Es
        // imprescindible consumir el stdout del hijo: si nadie lee y el pipe
        // se llena, el helper se bloquearía al escribir.
        //
        // Si el stdout se cierra (EOF) después de haber visto "READY" y sin
        // que la app haya pedido detener la captura, el helper murió a mitad
        // de grabación: se notifica a la UI en vez de dejarla creyendo que
        // sigue grabando.
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<()>();
        let app = self.app.clone();
        let stopping = self.stopping.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            let mut ready_sent = false;
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if !ready_sent && line.trim() == "READY" {
                    ready_sent = true;
                    let _ = ready_tx.send(());
                    continue;
                }
                if let Some(rest) = line.strip_prefix("LVL ") {
                    let mut parts = rest.trim().split_whitespace();
                    if let (Some(sys_str), Some(mic_str)) = (parts.next(), parts.next()) {
                        if let (Ok(system), Ok(mic)) =
                            (sys_str.parse::<f32>(), mic_str.parse::<f32>())
                        {
                            let _ = app.emit(LEVEL_EVENT, AudioLevels { system, mic });
                        }
                    }
                }
            }
            if ready_sent && !stopping.load(Ordering::SeqCst) {
                let _ = app.emit(
                    CRASH_EVENT,
                    "La captura de audio del sistema se interrumpió inesperadamente.",
                );
            }
        });

        // Esperar a que el helper confirme el arranque real ("READY") o a
        // que el canal se cierre porque stdout llegó a EOF sin haber enviado
        // "READY" (el proceso murió antes de arrancar, p. ej. permiso
        // denegado). El timeout cubre el caso de que se quede colgado sin
        // salir ni confirmar.
        let ready = tokio::task::spawn_blocking(move || ready_rx.recv_timeout(STARTUP_TIMEOUT))
            .await
            .unwrap_or(Err(std::sync::mpsc::RecvTimeoutError::Disconnected));

        if ready.is_err() {
            let _ = child.kill();
            let _ = child.wait();
            let mut err = String::new();
            if let Some(mut se) = child.stderr.take() {
                let _ = se.read_to_string(&mut err);
            }
            return Err(ResomerError::RecordingError(Self::describe_startup_error(
                &err,
            )));
        }

        *self.child.lock().unwrap() = Some(child);
        Ok(())
    }

    async fn stop_recording(&self) -> Result<(), ResomerError> {
        self.stopping.store(true, Ordering::SeqCst);
        let child = self.child.lock().unwrap().take();
        if let Some(mut child) = child {
            Self::terminate_gracefully(&mut child)?;
        }
        Ok(())
    }

    async fn pause_recording(&self) -> Result<(), ResomerError> {
        // ScreenCaptureKit no expone pausa nativa; detenemos la captura.
        // (El flujo de la app trata pausa como detener temporalmente.)
        self.stopping.store(true, Ordering::SeqCst);
        let child = self.child.lock().unwrap().take();
        if let Some(mut child) = child {
            Self::terminate_gracefully(&mut child)?;
        }
        Ok(())
    }
}
