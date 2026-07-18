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

    let input_devices = host
        .input_devices()
        .map_err(|e| ResomerError::RecordingError(format!("Failed to enumerate devices: {}", e)))?;

    for (device_index, device) in input_devices.enumerate() {
        let is_default = host
            .default_input_device()
            .as_ref()
            .map(|d| std::ptr::eq(d, &device))
            .unwrap_or(false);

        let channels = device
            .default_input_config()
            .ok()
            .map(|c| c.channels())
            .unwrap_or(0);

        // Use generic names since cpal doesn't expose device names reliably
        let name = format!("Device {}", device_index);

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
