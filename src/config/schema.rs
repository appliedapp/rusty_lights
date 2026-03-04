//! Configuration schema definitions

use serde::Deserialize;

/// Main configuration structure
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub audio: AudioConfig,
    pub dsp: DspConfig,
    pub effect: EffectConfig,
    pub output: OutputConfig,
    pub http: HttpConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            audio: AudioConfig::default(),
            dsp: DspConfig::default(),
            effect: EffectConfig::default(),
            output: OutputConfig::default(),
            http: HttpConfig::default(),
        }
    }
}

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
            leds: LedConfig::default(),
            segments: Vec::new(),
        }
    }
}

/// LED strip configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LedConfig {
    /// Total number of LEDs
    pub count: usize,
    /// RGB byte order: "RGB", "GRB", "BGR", etc.
    pub rgb_order: String,
}

impl Default for LedConfig {
    fn default() -> Self {
        Self {
            count: 300,
            rgb_order: "GRB".to_string(),
        }
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
