// SPDX-License-Identifier: GPL-3.0-only
// Copyright (c) 2026 appliedappliance GmbH
//! Digital Signal Processing pipeline
//!
//! This module provides the complete DSP chain for audio visualization:
//! - FFT for frequency analysis
//! - Mel filterbank for perceptual frequency bands
//! - Smoothing filters for temporal coherence
//! - Beat detection for rhythm-reactive effects

mod fft;
mod filters;
mod mel;
mod pipeline;

pub mod beat;

// Core processors
pub use fft::FftProcessor;
pub use mel::MelBank;

// Filters
pub use filters::{Agc, AttackReleaseSmoother, Smoother};

// Beat detection
pub use beat::BeatDetector;

// Unified pipeline
pub use pipeline::{DspConfig, DspPipeline, DspResult};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DspError {
    #[error("Invalid FFT size: {0}. Must be a power of 2.")]
    InvalidFftSize(usize),
    #[error("Invalid frequency range: min={0}, max={1}")]
    InvalidFrequencyRange(f32, f32),
    #[error("Processing error: {0}")]
    ProcessingError(String),
}
