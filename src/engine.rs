// SPDX-License-Identifier: GPL-3.0-only
// Copyright (c) 2026 appliedappliance GmbH
//! Core visualizer engine
//!
//! Orchestrates the full audio-to-LED pipeline.

use crate::audio::{AudioBackend, RingBuffer, create_backend};
use crate::config::Config;
use crate::dsp::{DspConfig, DspPipeline};
use crate::effects::{EffectRegistry, Rgb};
use crate::output::{RateLimiter, create_output};
use crate::util::reorder_rgb;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use thiserror::Error;

type FrameData = (Vec<Rgb>, Vec<f32>, Option<f32>);

/// Config update messages from the web UI
#[derive(Debug)]
pub enum ConfigUpdateMsg {
    SetEffect(String),
    SetBrightness(f32),
    SetSmoothing(f32),
    SetBeatSensitivity(f32),
}

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Audio error: {0}")]
    Audio(#[from] crate::audio::AudioError),
    #[error("DSP error: {0}")]
    Dsp(#[from] crate::dsp::DspError),
    #[error("Effect error: {0}")]
    Effect(#[from] crate::effects::EffectError),
    #[error("Output error: {0}")]
    Output(#[from] crate::output::OutputError),
    #[error("Thread error: {0}")]
    Thread(String),
}

/// Main visualizer engine
pub struct Engine {
    config: Config,
    running: Arc<AtomicBool>,
    audio_backend: Option<Box<dyn AudioBackend>>,
    ring_buffer: Arc<RingBuffer<f32>>,
    processing_thread: Option<JoinHandle<()>>,
    #[cfg(feature = "http")]
    web_thread: Option<JoinHandle<()>>,
}

impl Engine {
    /// Create a new engine from configuration
    pub fn new(config: Config) -> Result<Self, EngineError> {
        // Create ring buffer (8192 samples = ~170ms at 48kHz)
        let ring_buffer = Arc::new(RingBuffer::new(8192));

        Ok(Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
            audio_backend: None,
            ring_buffer,
            processing_thread: None,
            #[cfg(feature = "http")]
            web_thread: None,
        })
    }

    /// Start the visualizer
    pub fn start(&mut self) -> Result<(), EngineError> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(()); // Already running
        }

        self.running.store(true, Ordering::SeqCst);

        // Create and start audio backend
        let mut audio = create_backend(
            &self.config.audio.backend,
            self.config.audio.sample_rate,
            self.ring_buffer.clone(),
            self.config.audio.fifo_path.as_deref(),
        )?;

        // Set device if specified
        if self.config.audio.device != "auto" {
            audio.set_device(&self.config.audio.device)?;
        }

        audio.start()?;
        self.audio_backend = Some(audio);

        // Set up optional web server channels
        #[allow(unused_mut)]
        let mut frame_tx: Option<mpsc::Sender<FrameData>> = None;
        #[allow(unused_mut)]
        let mut config_rx: Option<mpsc::Receiver<ConfigUpdateMsg>> = None;

        #[cfg(feature = "http")]
        {
            if self.config.http.enabled {
                let (ftx, frx) = mpsc::channel();
                let (ctx, crx) = mpsc::channel();
                frame_tx = Some(ftx);
                config_rx = Some(crx);

                let web_handle = crate::web::start(
                    self.config.http.port,
                    frx,
                    ctx,
                    self.running.clone(),
                    self.config.effect.name.clone(),
                    self.config.effect.params.brightness.unwrap_or(1.0),
                    self.config.dsp.smoothing,
                    self.config.dsp.beat_sensitivity,
                )
                .map_err(|e| EngineError::Thread(e.to_string()))?;

                self.web_thread = Some(web_handle);
            }
        }

        // Start processing thread
        let config = self.config.clone();
        let ring_buffer = self.ring_buffer.clone();
        let running = self.running.clone();

        let handle = thread::Builder::new()
            .name("visualizer".to_string())
            .spawn(move || {
                if let Err(e) =
                    run_processing_loop(config, ring_buffer, running, frame_tx, config_rx)
                {
                    log::error!("Processing loop error: {}", e);
                }
            })
            .map_err(|e| EngineError::Thread(e.to_string()))?;

        self.processing_thread = Some(handle);

        log::info!("Engine started");
        Ok(())
    }

    /// Stop the visualizer
    pub fn stop(&mut self) -> Result<(), EngineError> {
        if !self.running.load(Ordering::SeqCst) {
            return Ok(()); // Already stopped
        }

        self.running.store(false, Ordering::SeqCst);

        // Stop audio backend
        if let Some(ref mut audio) = self.audio_backend {
            audio.stop()?;
        }
        self.audio_backend = None;

        // Wait for processing thread
        if let Some(handle) = self.processing_thread.take() {
            handle
                .join()
                .map_err(|_| EngineError::Thread("Thread join failed".to_string()))?;
        }

        // Wait for web server thread
        #[cfg(feature = "http")]
        {
            if let Some(handle) = self.web_thread.take() {
                let _ = handle.join();
            }
        }

        log::info!("Engine stopped");
        Ok(())
    }

    /// Check if engine is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get the running flag for external shutdown
    pub fn running_flag(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Main processing loop
fn run_processing_loop(
    config: Config,
    ring_buffer: Arc<RingBuffer<f32>>,
    running: Arc<AtomicBool>,
    frame_tx: Option<mpsc::Sender<FrameData>>,
    config_rx: Option<mpsc::Receiver<ConfigUpdateMsg>>,
) -> Result<(), EngineError> {
    // Initialize DSP pipeline
    let dsp_config = DspConfig {
        fft_size: config.dsp.fft_size,
        mel_bands: config.dsp.mel_bands,
        sample_rate: config.audio.sample_rate,
        freq_min: config.dsp.freq_min,
        freq_max: config.dsp.freq_max,
        smoothing: config.dsp.smoothing,
        beat_sensitivity: config.dsp.beat_sensitivity,
    };
    let mut dsp = DspPipeline::new(&dsp_config)?;

    // Initialize effect
    let registry = EffectRegistry::new();
    let mut effect = registry.create(&config.effect.name, config.output.leds.count)?;

    // Apply effect parameters
    if let Some(brightness) = config.effect.params.brightness {
        let _ = effect.set_param("brightness", brightness);
    }
    if let Some(speed) = config.effect.params.speed {
        let _ = effect.set_param("speed", speed);
    }

    // Initialize output
    let mut output = create_output(
        &config.output.protocol,
        &config.output.target,
        config.output.universe,
    )?;

    // Initialize rate limiter
    let mut rate_limiter = RateLimiter::new(config.output.fps);

    // Allocate buffers
    let fft_size = config.dsp.fft_size;
    let num_leds = config.output.leds.count;
    let rgb_order = config.output.leds.rgb_order.clone();

    let mut audio_buffer = vec![0.0f32; fft_size];
    let mut led_buffer = vec![Rgb::black(); num_leds];
    let mut dmx_buffer = vec![0u8; num_leds * 3];

    let mut frame_count: u64 = 0;

    log::info!(
        "Processing loop started: {} LEDs, {} FPS, {} effect",
        num_leds,
        config.output.fps,
        config.effect.name
    );

    // Main loop
    while running.load(Ordering::Relaxed) {
        // Handle config updates from web UI
        if let Some(ref rx) = config_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ConfigUpdateMsg::SetEffect(name) => match registry.create(&name, num_leds) {
                        Ok(new_effect) => {
                            effect = new_effect;
                            log::info!("Effect changed to: {}", name);
                        }
                        Err(e) => log::warn!("Failed to set effect '{}': {}", name, e),
                    },
                    ConfigUpdateMsg::SetBrightness(v) => {
                        let _ = effect.set_param("brightness", v);
                    }
                    ConfigUpdateMsg::SetSmoothing(v) => {
                        dsp.set_smoothing(v);
                    }
                    ConfigUpdateMsg::SetBeatSensitivity(v) => {
                        dsp.set_beat_sensitivity(v);
                    }
                }
            }
        }

        // Read audio samples from ring buffer
        let samples_read = ring_buffer.pop_slice(&mut audio_buffer);

        if samples_read >= fft_size {
            // Process through DSP pipeline
            let dsp_result = dsp.process(&audio_buffer);

            // Render effect
            effect.render(dsp_result.mel_bands, dsp_result.beat, &mut led_buffer);

            // Send frame to web UI (~30fps: every other frame)
            if let Some(ref tx) = frame_tx
                && frame_count.is_multiple_of(2)
            {
                let _ = tx.send((
                    led_buffer.clone(),
                    dsp_result.mel_bands.to_vec(),
                    dsp_result.beat,
                ));
            }

            // Convert to DMX data
            for (i, led) in led_buffer.iter().enumerate() {
                let offset = i * 3;
                dmx_buffer[offset] = led.r;
                dmx_buffer[offset + 1] = led.g;
                dmx_buffer[offset + 2] = led.b;
            }

            // Apply RGB reordering
            reorder_rgb(&mut dmx_buffer, &rgb_order);

            // Convert back to Rgb for output
            for (i, led) in led_buffer.iter_mut().enumerate() {
                let offset = i * 3;
                led.r = dmx_buffer[offset];
                led.g = dmx_buffer[offset + 1];
                led.b = dmx_buffer[offset + 2];
            }

            // Send to output
            if let Err(e) = output.send(&led_buffer) {
                log::warn!("Output send failed: {}", e);
            }

            frame_count += 1;
        }

        // Rate limiting
        rate_limiter.wait();
    }

    // Turn off LEDs on shutdown
    led_buffer.fill(Rgb::black());
    let _ = output.send(&led_buffer);

    log::info!(
        "Processing loop stopped. Frames: {}, Dropped: {}",
        rate_limiter.frame_count(),
        rate_limiter.dropped_frames()
    );

    Ok(())
}
