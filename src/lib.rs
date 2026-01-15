//! RustyLights - Resource-efficient audio-to-LED visualization
//!
//! This crate provides real-time audio analysis and LED control via E1.31/sACN and DDP protocols.

pub mod audio;
pub mod config;
pub mod dsp;
pub mod effects;
pub mod engine;
pub mod output;
pub mod util;

pub use config::Config;
pub use dsp::{DspConfig, DspPipeline};
pub use effects::Rgb;
pub use engine::Engine;
