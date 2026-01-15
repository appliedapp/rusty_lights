//! Audio capture backends and ring buffer
//!
//! Provides audio capture from PipeWire (preferred) or ALSA (fallback).

mod ringbuffer;

#[cfg(feature = "pipewire")]
mod pipewire;

#[cfg(feature = "alsa")]
mod alsa;

pub use ringbuffer::RingBuffer;

#[cfg(feature = "pipewire")]
pub use self::pipewire::{AudioDevice as PipeWireDevice, PipeWireCapture};

#[cfg(feature = "alsa")]
pub use self::alsa::{AlsaCapture, AlsaDevice};

use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Failed to initialize audio backend: {0}")]
    InitError(String),
    #[error("Device not found: {0}")]
    DeviceNotFound(String),
    #[error("Stream error: {0}")]
    StreamError(String),
    #[error("No audio backend available")]
    NoBackend,
}

/// Common trait for audio backends
pub trait AudioBackend: Send {
    /// Start capturing audio
    fn start(&mut self) -> Result<(), AudioError>;
    /// Stop capturing audio
    fn stop(&mut self) -> Result<(), AudioError>;
    /// Set the audio device
    fn set_device(&mut self, device: &str) -> Result<(), AudioError>;
    /// Get the current sample rate
    fn sample_rate(&self) -> u32;
}

/// Audio device information (unified across backends)
#[derive(Debug, Clone)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub description: String,
    pub backend: &'static str,
}

/// Create the appropriate audio backend based on available features and configuration
///
/// # Arguments
/// * `backend_name` - Backend to use: "pipewire", "alsa", or "auto"
/// * `sample_rate` - Desired sample rate
/// * `ring_buffer` - Shared ring buffer for audio samples
pub fn create_backend(
    backend_name: &str,
    sample_rate: u32,
    ring_buffer: Arc<RingBuffer<f32>>,
) -> Result<Box<dyn AudioBackend>, AudioError> {
    match backend_name.to_lowercase().as_str() {
        #[cfg(feature = "pipewire")]
        "pipewire" => {
            let capture = PipeWireCapture::new(sample_rate, ring_buffer)?;
            Ok(Box::new(capture))
        }

        #[cfg(feature = "alsa")]
        "alsa" => {
            let capture = AlsaCapture::new(sample_rate, ring_buffer)?;
            Ok(Box::new(capture))
        }

        "auto" => {
            // Try PipeWire first, then ALSA
            #[cfg(feature = "pipewire")]
            {
                match PipeWireCapture::new(sample_rate, ring_buffer.clone()) {
                    Ok(capture) => {
                        log::info!("Using PipeWire audio backend");
                        return Ok(Box::new(capture));
                    }
                    Err(e) => {
                        log::warn!("PipeWire not available: {}", e);
                    }
                }
            }

            #[cfg(feature = "alsa")]
            {
                match AlsaCapture::new(sample_rate, ring_buffer) {
                    Ok(capture) => {
                        log::info!("Using ALSA audio backend");
                        return Ok(Box::new(capture));
                    }
                    Err(e) => {
                        log::warn!("ALSA not available: {}", e);
                    }
                }
            }

            Err(AudioError::NoBackend)
        }

        _ => Err(AudioError::InitError(format!(
            "Unknown backend: {}. Available: pipewire, alsa, auto",
            backend_name
        ))),
    }
}

/// List all available audio devices across all backends
pub fn list_all_devices() -> Vec<AudioDeviceInfo> {
    let mut devices = Vec::new();

    #[cfg(feature = "pipewire")]
    {
        if let Ok(pw_devices) = PipeWireCapture::list_devices() {
            for dev in pw_devices {
                devices.push(AudioDeviceInfo {
                    name: dev.name,
                    description: dev.description,
                    backend: "pipewire",
                });
            }
        }
    }

    #[cfg(feature = "alsa")]
    {
        if let Ok(alsa_devices) = AlsaCapture::list_devices() {
            for dev in alsa_devices {
                devices.push(AudioDeviceInfo {
                    name: dev.name,
                    description: dev.description,
                    backend: "alsa",
                });
            }
        }
    }

    devices
}

/// Check which audio backends are available at compile time
pub fn available_backends() -> Vec<&'static str> {
    let mut backends = Vec::new();

    #[cfg(feature = "pipewire")]
    backends.push("pipewire");

    #[cfg(feature = "alsa")]
    backends.push("alsa");

    backends
}
