//! Mel-scale filterbank for perceptual frequency analysis

use super::DspError;

/// Mel-scale filterbank
///
/// Converts FFT magnitude spectrum to perceptually-weighted frequency bands
/// using triangular filters on the Mel scale.
pub struct MelBank {
    /// Filter weights for each band (sparse representation)
    filters: Vec<MelFilter>,
    /// Number of Mel bands
    num_bands: usize,
    /// Reusable output buffer
    output: Box<[f32]>,
}

/// Single Mel filter (triangular)
struct MelFilter {
    /// Starting FFT bin
    start_bin: usize,
    /// Ending FFT bin (exclusive)
    end_bin: usize,
    /// Filter weights for bins in range [start_bin, end_bin)
    weights: Box<[f32]>,
}

impl MelBank {
    /// Create a new Mel filterbank
    ///
    /// # Arguments
    /// * `num_bands` - Number of Mel frequency bands (typically 16-32)
    /// * `fft_size` - FFT size used for input
    /// * `sample_rate` - Audio sample rate in Hz
    /// * `freq_min` - Minimum frequency (Hz)
    /// * `freq_max` - Maximum frequency (Hz)
    pub fn new(
        num_bands: usize,
        fft_size: usize,
        sample_rate: u32,
        freq_min: f32,
        freq_max: f32,
    ) -> Result<Self, DspError> {
        if freq_min >= freq_max || freq_min < 0.0 {
            return Err(DspError::InvalidFrequencyRange(freq_min, freq_max));
        }

        let num_fft_bins = fft_size / 2 + 1;
        let filters = Self::create_filterbank(num_bands, num_fft_bins, sample_rate, freq_min, freq_max);

        Ok(Self {
            filters,
            num_bands,
            output: vec![0.0; num_bands].into_boxed_slice(),
        })
    }

    /// Convert frequency to Mel scale
    #[inline]
    fn hz_to_mel(hz: f32) -> f32 {
        2595.0 * (1.0 + hz / 700.0).log10()
    }

    /// Convert Mel scale to frequency
    #[inline]
    fn mel_to_hz(mel: f32) -> f32 {
        700.0 * (10.0_f32.powf(mel / 2595.0) - 1.0)
    }

    /// Create triangular filterbank
    fn create_filterbank(
        num_bands: usize,
        num_fft_bins: usize,
        sample_rate: u32,
        freq_min: f32,
        freq_max: f32,
    ) -> Vec<MelFilter> {
        let mel_min = Self::hz_to_mel(freq_min);
        let mel_max = Self::hz_to_mel(freq_max);

        // Create num_bands + 2 points (for triangular filter edges)
        let num_points = num_bands + 2;
        let mel_points: Vec<f32> = (0..num_points)
            .map(|i| mel_min + (mel_max - mel_min) * i as f32 / (num_points - 1) as f32)
            .collect();

        let hz_points: Vec<f32> = mel_points.iter().map(|&m| Self::mel_to_hz(m)).collect();

        // Convert Hz to FFT bin indices
        let bin_points: Vec<usize> = hz_points
            .iter()
            .map(|&hz| {
                let bin = (hz * num_fft_bins as f32 * 2.0 / sample_rate as f32).floor() as usize;
                bin.min(num_fft_bins - 1)
            })
            .collect();

        // Create triangular filters
        let mut filters = Vec::with_capacity(num_bands);

        for i in 0..num_bands {
            let start_bin = bin_points[i];
            let center_bin = bin_points[i + 1];
            let end_bin = bin_points[i + 2];

            if start_bin >= end_bin {
                // Degenerate filter, create minimal filter
                filters.push(MelFilter {
                    start_bin: center_bin,
                    end_bin: center_bin + 1,
                    weights: vec![1.0].into_boxed_slice(),
                });
                continue;
            }

            let mut weights = Vec::with_capacity(end_bin - start_bin);

            // Rising edge
            for bin in start_bin..center_bin {
                if center_bin > start_bin {
                    let weight = (bin - start_bin) as f32 / (center_bin - start_bin) as f32;
                    weights.push(weight);
                }
            }

            // Falling edge
            for bin in center_bin..end_bin {
                if end_bin > center_bin {
                    let weight = (end_bin - bin) as f32 / (end_bin - center_bin) as f32;
                    weights.push(weight);
                }
            }

            if weights.is_empty() {
                weights.push(1.0);
            }

            filters.push(MelFilter {
                start_bin,
                end_bin,
                weights: weights.into_boxed_slice(),
            });
        }

        filters
    }

    /// Apply filterbank to FFT magnitude spectrum
    ///
    /// # Arguments
    /// * `magnitude` - FFT magnitude spectrum
    ///
    /// # Returns
    /// Mel band energies
    pub fn process(&mut self, magnitude: &[f32]) -> &[f32] {
        for (i, filter) in self.filters.iter().enumerate() {
            let mut sum = 0.0;

            for (j, &weight) in filter.weights.iter().enumerate() {
                let bin = filter.start_bin + j;
                if bin < magnitude.len() {
                    sum += magnitude[bin] * weight;
                }
            }

            self.output[i] = sum;
        }

        &self.output
    }

    /// Get number of Mel bands
    #[inline]
    pub fn num_bands(&self) -> usize {
        self.num_bands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mel_conversion() {
        // Test Hz to Mel and back
        let test_freqs = [100.0, 500.0, 1000.0, 4000.0, 8000.0];

        for &freq in &test_freqs {
            let mel = MelBank::hz_to_mel(freq);
            let back = MelBank::mel_to_hz(mel);
            assert!((freq - back).abs() < 0.01, "Conversion failed for {} Hz", freq);
        }
    }

    #[test]
    fn test_filterbank_creation() {
        let mel_bank = MelBank::new(24, 512, 48000, 20.0, 18000.0);
        assert!(mel_bank.is_ok());

        let mel_bank = mel_bank.unwrap();
        assert_eq!(mel_bank.num_bands(), 24);
    }

    #[test]
    fn test_filterbank_processing() {
        let mut mel_bank = MelBank::new(16, 512, 48000, 20.0, 18000.0).unwrap();

        // Create fake magnitude spectrum
        let magnitude: Vec<f32> = (0..257).map(|i| (i as f32 / 257.0)).collect();

        let bands = mel_bank.process(&magnitude);
        assert_eq!(bands.len(), 16);

        // All bands should have some energy
        assert!(bands.iter().all(|&b| b >= 0.0));
    }
}
