//! Digital Signal Processing pipeline

mod fft;
mod filters;
mod mel;

pub mod beat;

pub use fft::FftProcessor;
pub use filters::Smoother;
pub use mel::MelBank;

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
