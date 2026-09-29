use crate::ResomerError;
use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    pub id: usize,
    pub name: String,
    pub is_default: bool,
    pub channels: u16,
    pub sample_rates: Vec<u32>,
}

pub fn list_input_devices() -> Result<Vec<AudioDevice>, ResomerError> {
    let host = cpal::default_host();

    let mut devices = Vec::new();

    // El dispositivo por defecto se compara por nombre: `default_input_device()`
    // devuelve una instancia nueva, así que comparar punteros (lo anterior)
    // nunca daba `true` y la UI no marcaba el dispositivo por defecto.
    let default_name = host.default_input_device().map(|d| d.to_string());

    let input_devices = host
        .input_devices()
        .map_err(|e| ResomerError::RecordingError(format!("Failed to enumerate devices: {}", e)))?;

    for (device_index, device) in input_devices.enumerate() {
        let name = device.to_string();
        let is_default = default_name.as_deref() == Some(name.as_str());

        let channels = device
            .default_input_config()
            .ok()
            .map(|c| c.channels())
            .unwrap_or(0);

        // Collect common sample rates
        let sample_rates = vec![8000, 16000, 44100, 48000];

        devices.push(AudioDevice {
            id: device_index,
            name,
            is_default,
            channels,
            sample_rates,
        });
    }

    if devices.is_empty() {
        return Err(ResomerError::RecordingError(
            "No input devices found".to_string(),
        ));
    }

    Ok(devices)
}
