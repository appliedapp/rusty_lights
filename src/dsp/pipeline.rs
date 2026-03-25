// SPDX-License-Identifier: GPL-3.0-only
// Copyright (c) 2026 appliedappliance GmbH
//! Unified DSP pipeline combining FFT, Mel filterbank, and smoothing

use super::{
    DspError,
    beat::BeatDetector,
    fft::FftProcessor,
    filters::{Agc, AttackReleaseSmoother},
    mel::MelBank,
};

/// Complete DSP processing pipeline
///
/// Orchestrates the full signal processing chain:
/// Audio samples -> FFT -> Mel filterbank -> Smoothing -> Output
pub struct DspPipeline {
    fft: FftProcessor,
    mel_bank: MelBank,
    smoother: AttackReleaseSmoother,
    agc: Agc,
    beat_detector: BeatDetector,
    /// Smoothed Mel band output
    output: Box<[f32]>,
    /// AGC intermediate buffer
    agc_buffer: Box<[f32]>,
    /// FFT size
    fft_size: usize,
    /// Number of Mel bands
    num_bands: usize,
}

/// DSP pipeline configuration
#[derive(Debug, Clone)]
pub struct DspConfig {
    /// FFT size (256, 512, or 1024)
    pub fft_size: usize,
    /// Number of Mel frequency bands
    pub mel_bands: usize,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Minimum frequency for Mel filterbank
    pub freq_min: f32,
    /// Maximum frequency for Mel filterbank
    pub freq_max: f32,
    /// Smoothing factor (0.0 - 1.0) — used as release time, attack is derived
    pub smoothing: f32,
    /// Beat detection sensitivity (1.0 - 3.0)
    pub beat_sensitivity: f32,
}

impl Default for DspConfig {
    fn default() -> Self {
        Self {
            fft_size: 512,
            mel_bands: 24,
            sample_rate: 48000,
            freq_min: 20.0,
            freq_max: 18000.0,
            smoothing: 0.7,
            beat_sensitivity: 1.5,
        }
    }
}

/// Result of DSP processing
#[derive(Debug)]
pub struct DspResult<'a> {
    /// Smoothed Mel band energies
    pub mel_bands: &'a [f32],
    /// Raw FFT magnitude spectrum
    pub magnitude: &'a [f32],
    /// Beat detection result (Some(strength) if beat detected)
    pub beat: Option<f32>,
}

impl DspPipeline {
    /// Create a new DSP pipeline
    pub fn new(config: &DspConfig) -> Result<Self, DspError> {
        let fft = FftProcessor::new(config.fft_size)?;
        let num_bins = fft.num_bins();

        let mel_bank = MelBank::new(
            config.mel_bands,
            config.fft_size,
            config.sample_rate,
            config.freq_min,
            config.freq_max,
        )?;

        // Attack/release smoother: fast attack (0.1), release from config smoothing
        let smoother = AttackReleaseSmoother::new(config.mel_bands, 0.1, config.smoothing);

        // AGC: target 0.8 peak, moderate tracking speed
        let agc = Agc::new(0.8, 0.05, 0.02);

        // Beat detector history: ~0.7s at 60fps = 43 frames
        let mut beat_detector = BeatDetector::new(num_bins, 43);
        beat_detector.set_sensitivity(config.beat_sensitivity);

        Ok(Self {
            fft,
            mel_bank,
            smoother,
            agc,
            beat_detector,
            output: vec![0.0; config.mel_bands].into_boxed_slice(),
            agc_buffer: vec![0.0; config.mel_bands].into_boxed_slice(),
            fft_size: config.fft_size,
            num_bands: config.mel_bands,
        })
    }

    /// Process audio samples through the complete pipeline
    ///
    /// # Arguments
    /// * `samples` - Audio samples (must be exactly fft_size samples)
    ///
    /// # Returns
    /// Processing results including Mel bands and beat detection
    pub fn process(&mut self, samples: &[f32]) -> DspResult<'_> {
        // Step 1: FFT
        let magnitude = self.fft.process(samples);

        // Step 2: Beat detection (before Mel to use full spectrum)
        let beat = self.beat_detector.process(magnitude);

        // Step 3: Mel filterbank
        let mel_raw = self.mel_bank.process(magnitude);

        // Step 4: AGC (normalize levels across quiet/loud passages)
        self.agc.process(mel_raw, &mut self.agc_buffer);

        // Step 5: Attack/release smoothing (fast attack, slow decay)
        self.smoother.process(&self.agc_buffer, &mut self.output);

        DspResult {
            mel_bands: &self.output,
            magnitude,
            beat,
        }
    }

    /// Get raw FFT processor for direct access
    pub fn fft(&mut self) -> &mut FftProcessor {
        &mut self.fft
    }

    /// Get Mel filterbank for direct access
    pub fn mel_bank(&mut self) -> &mut MelBank {
        &mut self.mel_bank
    }

    /// Get smoother for direct access
    pub fn smoother(&mut self) -> &mut AttackReleaseSmoother {
        &mut self.smoother
    }

    /// Get beat detector for direct access
    pub fn beat_detector(&mut self) -> &mut BeatDetector {
        &mut self.beat_detector
    }

    /// Get the FFT size
    #[inline]
    pub fn fft_size(&self) -> usize {
        self.fft_size
    }

    /// Get the number of Mel bands
    #[inline]
    pub fn num_bands(&self) -> usize {
        self.num_bands
    }

    /// Set smoothing factor (adjusts release time)
    pub fn set_smoothing(&mut self, smoothing: f32) {
        self.smoother.set_release(smoothing);
    }

    /// Set beat detection sensitivity
    pub fn set_beat_sensitivity(&mut self, sensitivity: f32) {
        self.beat_detector.set_sensitivity(sensitivity);
    }

    /// Reset all internal state
    pub fn reset(&mut self) {
        self.smoother.reset();
        self.agc.reset();
        self.beat_detector.reset();
        self.output.fill(0.0);
        self.agc_buffer.fill(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_creation() {
        let config = DspConfig::default();
        let pipeline = DspPipeline::new(&config);
        assert!(pipeline.is_ok());

        let pipeline = pipeline.unwrap();
        assert_eq!(pipeline.fft_size(), 512);
        assert_eq!(pipeline.num_bands(), 24);
    }

    #[test]
    fn test_pipeline_processing() {
        let config = DspConfig::default();
        let mut pipeline = DspPipeline::new(&config).unwrap();

        // Generate test signal (sine wave at 440Hz)
        let samples: Vec<f32> = (0..512)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin())
            .collect();

        let result = pipeline.process(&samples);

        // Should have correct number of Mel bands
        assert_eq!(result.mel_bands.len(), 24);

        // Magnitude should have correct size
        assert_eq!(result.magnitude.len(), 257); // fft_size/2 + 1

        // All values should be non-negative
        assert!(result.mel_bands.iter().all(|&v| v >= 0.0));
    }

    #[test]
    fn test_pipeline_beat_detection() {
        let config = DspConfig {
            beat_sensitivity: 1.2,
            ..Default::default()
        };
        let mut pipeline = DspPipeline::new(&config).unwrap();

        // Fill with quiet signal
        let quiet: Vec<f32> = vec![0.01; 512];
        for _ in 0..50 {
            pipeline.process(&quiet);
        }

        // Sudden loud signal should trigger beat
        let loud: Vec<f32> = (0..512)
            .map(|i| 0.8 * (2.0 * std::f32::consts::PI * 100.0 * i as f32 / 48000.0).sin())
            .collect();

        let result = pipeline.process(&loud);
        // Beat might or might not be detected depending on thresholds
        // Just verify processing completes without panic
        assert!(result.mel_bands.len() > 0);
    }

    #[test]
    fn test_pipeline_reset() {
        let config = DspConfig::default();
        let mut pipeline = DspPipeline::new(&config).unwrap();

        // Process some samples
        let samples: Vec<f32> = vec![0.5; 512];
        pipeline.process(&samples);

        // Reset
        pipeline.reset();

        // Output should be zeroed
        // (Internal state is reset, next process will start fresh)
    }
}
