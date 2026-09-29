//! Cliente HTTP compartido por todos los servicios que hablan con el gateway.
//!
//! Antes cada servicio creaba `reqwest::Client::new()` sin timeouts, así que un
//! gateway colgado dejaba el pipeline (transcripción/resumen/chat) esperando
//! para siempre sin forma de recuperarse.

use reqwest::Client;
use std::time::Duration;

/// Timeout global por petición. Es generoso porque Whisper puede tardar, pero
/// acotado: preferimos fallar con un error claro antes que colgarnos.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(180);

/// Timeout de conexión (DNS + TCP + TLS).
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

/// Cliente con los timeouts por defecto.
pub fn build_client() -> Client {
    build_client_with_timeout(DEFAULT_TIMEOUT)
}

/// Cliente con un timeout de petición concreto (p. ej. transcripción, que es
/// más lenta que el resto). Si la construcción falla, cae al cliente por
/// defecto para no romper el arranque.
pub fn build_client_with_timeout(timeout: Duration) -> Client {
    Client::builder()
        .timeout(timeout)
        .connect_timeout(CONNECT_TIMEOUT)
        .build()
        .unwrap_or_else(|_| Client::new())
}
