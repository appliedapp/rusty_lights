// SPDX-License-Identifier: GPL-3.0-only
// Copyright (c) 2026 appliedappliance GmbH
//! Beat detection using spectral flux

/// Beat detector using onset detection
///
/// Detects beats by analyzing spectral flux (changes in the frequency spectrum).
pub struct BeatDetector {
    /// Previous magnitude spectrum for flux calculation
    prev_magnitude: Box<[f32]>,
    /// Flux history for adaptive threshold
    flux_history: Box<[f32]>,
    /// Current position in flux history
    history_pos: usize,
    /// Threshold multiplier
    threshold_multiplier: f32,
    /// Minimum time between beats (in frames)
    min_beat_interval: usize,
    /// Frames since last beat
    frames_since_beat: usize,
}

impl BeatDetector {
    /// Create a new beat detector
    ///
    /// # Arguments
    /// * `num_bins` - Number of FFT bins to analyze
    /// * `history_size` - Number of frames for adaptive threshold (typically 43 for ~0.7s at 60fps)
    pub fn new(num_bins: usize, history_size: usize) -> Self {
        Self {
            prev_magnitude: vec![0.0; num_bins].into_boxed_slice(),
            flux_history: vec![0.0; history_size].into_boxed_slice(),
            history_pos: 0,
            threshold_multiplier: 1.5,
            min_beat_interval: 6, // ~100ms at 60fps
            frames_since_beat: 0,
        }
    }

    /// Process magnitude spectrum and detect beat
    ///
    /// # Arguments
    /// * `magnitude` - Current FFT magnitude spectrum
    ///
    /// # Returns
    /// Beat strength (0.0 = no beat, >0 = beat detected with intensity)
    pub fn process(&mut self, magnitude: &[f32]) -> Option<f32> {
        self.frames_since_beat += 1;

        // Calculate spectral flux (only positive changes = onsets)
        let mut flux = 0.0;
        for (i, &mag) in magnitude.iter().enumerate() {
            if i < self.prev_magnitude.len() {
                let diff = mag - self.prev_magnitude[i];
                if diff > 0.0 {
                    flux += diff;
                }
                self.prev_magnitude[i] = mag;
            }
        }

        // Store flux in history
        self.flux_history[self.history_pos] = flux;
        self.history_pos = (self.history_pos + 1) % self.flux_history.len();

        // Calculate adaptive threshold
        let mean_flux: f32 = self.flux_history.iter().sum::<f32>() / self.flux_history.len() as f32;
        let threshold = mean_flux * self.threshold_multiplier;

        // Detect beat
        if flux > threshold && self.frames_since_beat >= self.min_beat_interval {
            self.frames_since_beat = 0;
            let strength = (flux - threshold) / threshold.max(0.001);
            Some(strength.min(1.0))
        } else {
            None
        }
    }

    /// Set threshold sensitivity
    ///
    /// Higher values = less sensitive (fewer false positives)
    /// Lower values = more sensitive (more beats detected)
    pub fn set_sensitivity(&mut self, multiplier: f32) {
        self.threshold_multiplier = multiplier.clamp(1.0, 3.0);
    }

    /// Set minimum time between beats
    pub fn set_min_interval(&mut self, frames: usize) {
        self.min_beat_interval = frames.max(1);
    }

    /// Reset detector state
    pub fn reset(&mut self) {
        self.prev_magnitude.fill(0.0);
        self.flux_history.fill(0.0);
        self.history_pos = 0;
        self.frames_since_beat = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beat_detector_creation() {
        let detector = BeatDetector::new(257, 43);
        assert_eq!(detector.prev_magnitude.len(), 257);
        assert_eq!(detector.flux_history.len(), 43);
    }

    #[test]
    fn test_no_beat_on_silence() {
        let mut detector = BeatDetector::new(257, 43);
        let silence = vec![0.0; 257];

        // Should not detect beats in silence
        for _ in 0..100 {
            assert!(detector.process(&silence).is_none());
        }
    }

    #[test]
    fn test_beat_on_sudden_onset() {
        let mut detector = BeatDetector::new(257, 10);

        // Fill history with low values
        let low = vec![0.1; 257];
        for _ in 0..20 {
            detector.process(&low);
        }

        // Sudden loud onset should trigger beat
        let loud = vec![1.0; 257];
        let beat = detector.process(&loud);

        // Should detect a beat
        assert!(beat.is_some());
    }
}
