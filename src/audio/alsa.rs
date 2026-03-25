// SPDX-License-Identifier: GPL-3.0-only
// Copyright (c) 2026 appliedappliance GmbH
//! ALSA audio capture backend (fallback for systems without PipeWire)

use super::{AudioBackend, AudioError, RingBuffer};
use std::ffi::CString;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

use alsa::pcm::{Access, Format, HwParams, PCM};
use alsa::{Direction, ValueOr};

/// ALSA audio capture backend
///
/// Captures audio from an ALSA device (typically a loopback device).
pub struct AlsaCapture {
    sample_rate: u32,
    channels: u32,
    period_size: u32,
    ring_buffer: Arc<RingBuffer<f32>>,
    device: String,
    running: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

impl AlsaCapture {
    /// Create a new ALSA capture instance
    ///
    /// # Arguments
    /// * `sample_rate` - Desired sample rate (e.g., 48000)
    /// * `ring_buffer` - Shared ring buffer for audio samples
    pub fn new(sample_rate: u32, ring_buffer: Arc<RingBuffer<f32>>) -> Result<Self, AudioError> {
        Ok(Self {
            sample_rate,
            channels: 2,
            period_size: 512,
            ring_buffer,
            device: "default".to_string(),
            running: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
        })
    }

    /// List available ALSA capture devices
    pub fn list_devices() -> Result<Vec<AlsaDevice>, AudioError> {
        let mut devices = Vec::new();

        // Add common devices
        devices.push(AlsaDevice {
            name: "default".to_string(),
            description: "Default ALSA device".to_string(),
        });

        devices.push(AlsaDevice {
            name: "hw:Loopback,1".to_string(),
            description: "ALSA Loopback device (capture side)".to_string(),
        });

        // Try to enumerate PCM devices
        let iface = CString::new("pcm").unwrap();
        if let Ok(hints) = alsa::device_name::HintIter::new(None, &iface) {
            for hint in hints {
                if let (Some(name), Some(desc)) = (hint.name, hint.desc) {
                    // Only include capture devices
                    if hint.direction != Some(alsa::Direction::Playback) {
                        devices.push(AlsaDevice {
                            name,
                            description: desc.replace('\n', " "),
                        });
                    }
                }
            }
        }

        Ok(devices)
    }

    /// Set the period size (buffer size for each read)
    pub fn set_period_size(&mut self, size: u32) {
        self.period_size = size;
    }
}

impl AudioBackend for AlsaCapture {
    fn start(&mut self) -> Result<(), AudioError> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.running.store(true, Ordering::SeqCst);

        let sample_rate = self.sample_rate;
        let channels = self.channels;
        let period_size = self.period_size;
        let ring_buffer = self.ring_buffer.clone();
        let running = self.running.clone();
        let device = self.device.clone();

        // Spawn audio capture thread
        let handle = thread::Builder::new()
            .name("alsa-audio".to_string())
            .spawn(move || {
                if let Err(e) = run_capture_loop(
                    sample_rate,
                    channels,
                    period_size,
                    ring_buffer,
                    running,
                    &device,
                ) {
                    log::error!("ALSA capture error: {}", e);
                }
            })
            .map_err(|e| AudioError::InitError(format!("Failed to spawn thread: {}", e)))?;

        self.thread_handle = Some(handle);

        log::info!(
            "ALSA capture started: {}Hz, {} channels, device: {}",
            self.sample_rate,
            self.channels,
            self.device
        );

        Ok(())
    }

    fn stop(&mut self) -> Result<(), AudioError> {
        if !self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.running.store(false, Ordering::SeqCst);

        // Wait for thread to finish
        if let Some(handle) = self.thread_handle.take() {
            handle
                .join()
                .map_err(|_| AudioError::StreamError("Thread join failed".to_string()))?;
        }

        log::info!("ALSA capture stopped");
        Ok(())
    }

    fn set_device(&mut self, device: &str) -> Result<(), AudioError> {
        self.device = device.to_string();
        log::debug!("ALSA device set to: {}", device);
        Ok(())
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

impl Drop for AlsaCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// ALSA device information
#[derive(Debug, Clone)]
pub struct AlsaDevice {
    pub name: String,
    pub description: String,
}

const MAX_INIT_RETRIES: u32 = 12;
const RETRY_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

/// Open and configure the ALSA PCM device, retrying if the device is not yet ready
fn open_pcm(
    device: &str,
    sample_rate: u32,
    channels: u32,
    period_size: u32,
    running: &AtomicBool,
) -> Result<PCM, AudioError> {
    for attempt in 1..=MAX_INIT_RETRIES {
        if !running.load(Ordering::Relaxed) {
            return Err(AudioError::InitError(
                "Shutdown requested during init".to_string(),
            ));
        }

        match try_open_pcm(device, sample_rate, channels, period_size) {
            Ok(pcm) => return Ok(pcm),
            Err(e) => {
                if attempt == MAX_INIT_RETRIES {
                    return Err(e);
                }
                log::warn!(
                    "ALSA init attempt {}/{} failed: {}. Retrying in {}s...",
                    attempt,
                    MAX_INIT_RETRIES,
                    e,
                    RETRY_INTERVAL.as_secs()
                );
                std::thread::sleep(RETRY_INTERVAL);
            }
        }
    }
    unreachable!()
}

/// Single attempt to open and configure PCM
fn try_open_pcm(
    device: &str,
    sample_rate: u32,
    channels: u32,
    period_size: u32,
) -> Result<PCM, AudioError> {
    let pcm = PCM::new(device, Direction::Capture, false).map_err(|e| {
        AudioError::InitError(format!("Failed to open ALSA device '{}': {}", device, e))
    })?;

    {
        let hwp = HwParams::any(&pcm)
            .map_err(|e| AudioError::InitError(format!("Failed to get HW params: {}", e)))?;

        hwp.set_access(Access::RWInterleaved)
            .map_err(|e| AudioError::InitError(format!("Failed to set access: {}", e)))?;

        hwp.set_format(Format::s16())
            .map_err(|e| AudioError::InitError(format!("Failed to set format: {}", e)))?;

        hwp.set_channels(channels)
            .map_err(|e| AudioError::InitError(format!("Failed to set channels: {}", e)))?;

        if let Err(e) = hwp.set_rate(sample_rate, ValueOr::Nearest) {
            let min = hwp.get_rate_min().map_err(|e2| {
                AudioError::InitError(format!(
                    "Failed to set sample rate ({}) and cannot query min rate: {}",
                    e, e2
                ))
            })?;
            let max = hwp.get_rate_max().map_err(|e2| {
                AudioError::InitError(format!(
                    "Failed to set sample rate ({}) and cannot query max rate: {}",
                    e, e2
                ))
            })?;
            log::warn!(
                "Cannot set {}Hz ({}), device supports {}–{}Hz. Using {}Hz.",
                sample_rate,
                e,
                min,
                max,
                min
            );
            hwp.set_rate(min, ValueOr::Nearest).map_err(|e2| {
                AudioError::InitError(format!(
                    "Failed to set fallback sample rate {}Hz: {}",
                    min, e2
                ))
            })?;
        }

        let actual_rate = hwp
            .get_rate()
            .map_err(|e| AudioError::InitError(format!("Failed to get actual rate: {}", e)))?;
        if actual_rate != sample_rate {
            log::warn!(
                "ALSA negotiated sample rate {}Hz (requested {}Hz)",
                actual_rate,
                sample_rate
            );
        }

        hwp.set_period_size(period_size as alsa::pcm::Frames, ValueOr::Nearest)
            .map_err(|e| AudioError::InitError(format!("Failed to set period size: {}", e)))?;

        hwp.set_buffer_size((period_size * 4) as alsa::pcm::Frames)
            .map_err(|e| AudioError::InitError(format!("Failed to set buffer size: {}", e)))?;

        pcm.hw_params(&hwp)
            .map_err(|e| AudioError::InitError(format!("Failed to apply HW params: {}", e)))?;
    }

    pcm.prepare()
        .map_err(|e| AudioError::InitError(format!("Failed to prepare PCM: {}", e)))?;

    Ok(pcm)
}

/// Run the ALSA capture loop
fn run_capture_loop(
    sample_rate: u32,
    channels: u32,
    period_size: u32,
    ring_buffer: Arc<RingBuffer<f32>>,
    running: Arc<AtomicBool>,
    device: &str,
) -> Result<(), AudioError> {
    let pcm = open_pcm(device, sample_rate, channels, period_size, &running)?;

    // Create buffer for reading samples
    let buffer_size = (period_size * channels) as usize;
    let mut buffer: Vec<i16> = vec![0; buffer_size];

    log::info!("ALSA capture ready, entering main loop");

    // Main capture loop
    while running.load(Ordering::Relaxed) {
        // Read samples from ALSA
        let io = pcm
            .io_i16()
            .map_err(|e| AudioError::StreamError(e.to_string()))?;

        match io.readi(&mut buffer) {
            Ok(frames) => {
                // Convert i16 to f32 and mix to mono
                let samples_read = frames * channels as usize;

                if channels == 2 {
                    for chunk in buffer[..samples_read].chunks(2) {
                        if chunk.len() == 2 {
                            let left = chunk[0] as f32 / 32768.0;
                            let right = chunk[1] as f32 / 32768.0;
                            let mono = (left + right) * 0.5;
                            ring_buffer.push(mono);
                        }
                    }
                } else {
                    for &sample in &buffer[..samples_read] {
                        let normalized = sample as f32 / 32768.0;
                        ring_buffer.push(normalized);
                    }
                }
            }
            Err(e) => {
                // Handle underrun/overrun - EPIPE = -32
                let errno = e.errno();
                if errno == -32 {
                    log::warn!("ALSA buffer overrun, recovering...");
                    pcm.prepare().ok();
                    continue;
                }
                log::error!("ALSA read error: {} (errno: {})", e, errno);
                break;
            }
        }
    }

    // Stop and close
    pcm.drop()
        .map_err(|e| AudioError::StreamError(format!("Failed to stop PCM: {}", e)))?;

    Ok(())
}
