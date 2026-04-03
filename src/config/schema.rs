// SPDX-License-Identifier: GPL-3.0-only
// Copyright (c) 2026 appliedappliance GmbH
//! Configuration schema definitions

use serde::Deserialize;

/// Main configuration structure
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub audio: AudioConfig,
    pub dsp: DspConfig,
    pub effect: EffectConfig,
    pub output: OutputConfig,
    pub http: HttpConfig,
}

// Default is derived via serde(default) on all fields

/// Audio capture configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AudioConfig {
    /// Audio backend: "pipewire", "alsa", or "fifo"
    pub backend: String,
    /// Device name or "auto" for automatic selection
    pub device: String,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Audio chunk size in samples
    pub chunk_size: usize,
    /// Path to FIFO named pipe (used when backend = "fifo")
    pub fifo_path: Option<String>,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            backend: "pipewire".to_string(),
            device: "auto".to_string(),
            sample_rate: 48000,
            chunk_size: 512,
            fifo_path: None,
        }
    }
}

/// DSP processing configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DspConfig {
    /// FFT size (256, 512, or 1024)
    pub fft_size: usize,
    /// Number of Mel frequency bands
    pub mel_bands: usize,
    /// Minimum frequency for Mel bank (Hz)
    pub freq_min: f32,
    /// Maximum frequency for Mel bank (Hz)
    pub freq_max: f32,
    /// Smoothing factor (0.0 - 1.0)
    pub smoothing: f32,
    /// Beat detection sensitivity (1.0 - 3.0, higher = fewer beats)
    pub beat_sensitivity: f32,
}

impl Default for DspConfig {
    fn default() -> Self {
        Self {
            fft_size: 512,
            mel_bands: 24,
            freq_min: 20.0,
            freq_max: 18000.0,
            smoothing: 0.7,
            beat_sensitivity: 1.5,
        }
    }
}

/// Effect configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct EffectConfig {
    /// Effect name
    pub name: String,
    /// Effect-specific parameters
    pub params: EffectParams,
}

impl Default for EffectConfig {
    fn default() -> Self {
        Self {
            name: "spectrum".to_string(),
            params: EffectParams::default(),
        }
    }
}

/// Effect-specific parameters
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct EffectParams {
    /// Color gradient name
    pub gradient: Option<String>,
    /// Mirror the effect
    pub mirror: Option<bool>,
    /// Effect speed multiplier
    pub speed: Option<f32>,
    /// Brightness (0.0 - 1.0)
    pub brightness: Option<f32>,
}

/// Output configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct OutputConfig {
    /// Output protocol: "e131", "ddp", or "artnet"
    pub protocol: String,
    /// Target address (multicast or unicast IP)
    pub target: String,
    /// Starting universe number (for E1.31)
    pub universe: u16,
    /// Target frames per second
    pub fps: u32,
    /// Idle timeout in minutes (0 = disabled). Closes the connection when no
    /// audio is received for this duration and reconnects when audio resumes.
    pub idle_timeout: u64,
    /// LED configuration
    pub leds: LedConfig,
    /// Optional output segments for multi-controller setups
    #[serde(default)]
    pub segments: Vec<OutputSegment>,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            protocol: "e131".to_string(),
            target: "239.255.0.1".to_string(),
            universe: 1,
            fps: 60,
            idle_timeout: 0,
            leds: LedConfig::default(),
            segments: Vec::new(),
        }
    }
}

/// LED strip configuration
///
/// Fields are optional so that values not explicitly set in the config
/// can be auto-detected from WLED devices.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct LedConfig {
    /// Total number of LEDs (auto-detected from WLED when absent)
    pub count: Option<usize>,
    /// RGB byte order: "RGB", "GRB", "BGR", etc. (auto-detected from WLED when absent)
    pub rgb_order: Option<String>,
}

impl LedConfig {
    /// Resolved LED count, falling back to 300 if not configured.
    pub fn led_count(&self) -> usize {
        self.count.unwrap_or(300)
    }

    /// Resolved RGB order, falling back to "GRB" if not configured.
    pub fn order(&self) -> &str {
        self.rgb_order.as_deref().unwrap_or("GRB")
    }
}

/// HTTP server configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct HttpConfig {
    /// Enable the HTTP/WebSocket server
    pub enabled: bool,
    /// Port to listen on
    pub port: u16,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 8080,
        }
    }
}

/// Output segment for multi-controller setups
#[derive(Debug, Clone, Deserialize)]
pub struct OutputSegment {
    /// Target address for this segment
    pub target: String,
    /// Universe number
    pub universe: u16,
    /// Starting LED index
    pub start_led: usize,
    /// Ending LED index
    pub end_led: usize,
}
