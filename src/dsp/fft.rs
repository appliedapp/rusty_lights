//! FFT processing with windowing

use realfft::{num_complex::Complex, RealFftPlanner, RealToComplex};
use std::sync::Arc;

use super::DspError;

/// FFT processor with Hanning window
pub struct FftProcessor {
    fft: Arc<dyn RealToComplex<f32>>,
    window: Box<[f32]>,
    input_buffer: Box<[f32]>,
    scratch: Box<[Complex<f32>]>,
    output_buffer: Box<[Complex<f32>]>,
    magnitude: Box<[f32]>,
    fft_size: usize,
}

impl FftProcessor {
    /// Create a new FFT processor
    ///
    /// # Arguments
    /// * `fft_size` - FFT size (must be power of 2: 256, 512, 1024)
    pub fn new(fft_size: usize) -> Result<Self, DspError> {
        if !fft_size.is_power_of_two() || fft_size < 64 {
            return Err(DspError::InvalidFftSize(fft_size));
        }

        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(fft_size);

        let scratch_len = fft.get_scratch_len();
        let output_len = fft_size / 2 + 1;

        // Pre-compute Hanning window coefficients
        let window = Self::create_hanning_window(fft_size);

        Ok(Self {
            fft,
            window,
            input_buffer: vec![0.0; fft_size].into_boxed_slice(),
            scratch: vec![Complex::new(0.0, 0.0); scratch_len].into_boxed_slice(),
            output_buffer: vec![Complex::new(0.0, 0.0); output_len].into_boxed_slice(),
            magnitude: vec![0.0; output_len].into_boxed_slice(),
            fft_size,
        })
    }

    /// Create a Hanning window of the specified size
    fn create_hanning_window(size: usize) -> Box<[f32]> {
        let mut window = Vec::with_capacity(size);
        let factor = 2.0 * std::f32::consts::PI / (size - 1) as f32;

        for i in 0..size {
            let value = 0.5 * (1.0 - (factor * i as f32).cos());
            window.push(value);
        }

        window.into_boxed_slice()
    }

    /// Process audio samples and return magnitude spectrum
    ///
    /// # Arguments
    /// * `samples` - Input audio samples (must be exactly fft_size samples)
    ///
    /// # Returns
    /// Magnitude spectrum (fft_size/2 + 1 bins)
    pub fn process(&mut self, samples: &[f32]) -> &[f32] {
        debug_assert_eq!(samples.len(), self.fft_size);

        // Apply window function
        for (i, (&sample, &win)) in samples.iter().zip(self.window.iter()).enumerate() {
            self.input_buffer[i] = sample * win;
        }

        // Perform FFT
        self.fft
            .process_with_scratch(
                &mut self.input_buffer,
                &mut self.output_buffer,
                &mut self.scratch,
            )
            .expect("FFT processing failed");

        // Calculate magnitude (avoiding sqrt for performance)
        // Using magnitude squared is often sufficient for visualization
        let scale = 1.0 / self.fft_size as f32;
        for (i, c) in self.output_buffer.iter().enumerate() {
            // Magnitude: sqrt(re^2 + im^2), but we use re^2 + im^2 for speed
            // Then take sqrt only when needed for display
            self.magnitude[i] = (c.re * c.re + c.im * c.im).sqrt() * scale;
        }

        &self.magnitude
    }

    /// Get the FFT size
    #[inline]
    pub fn fft_size(&self) -> usize {
        self.fft_size
    }

    /// Get the number of frequency bins
    #[inline]
    pub fn num_bins(&self) -> usize {
        self.magnitude.len()
    }

    /// Get the frequency for a given bin index
    #[inline]
    pub fn bin_to_frequency(&self, bin: usize, sample_rate: u32) -> f32 {
        bin as f32 * sample_rate as f32 / self.fft_size as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fft_creation() {
        let fft = FftProcessor::new(512);
        assert!(fft.is_ok());

        let fft = FftProcessor::new(100); // Not power of 2
        assert!(fft.is_err());
    }

    #[test]
    fn test_fft_processing() {
        let mut fft = FftProcessor::new(512).unwrap();

        // Generate a simple sine wave at 1kHz (assuming 48kHz sample rate)
        let samples: Vec<f32> = (0..512)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 48000.0).sin())
            .collect();

        let magnitude = fft.process(&samples);

        // Check that we get non-zero output
        assert!(magnitude.iter().any(|&m| m > 0.0));
    }

    #[test]
    fn test_bin_to_frequency() {
        let fft = FftProcessor::new(512).unwrap();

        // At 48kHz sample rate, bin 0 = 0 Hz, bin 256 = 24kHz
        assert_eq!(fft.bin_to_frequency(0, 48000), 0.0);
        assert!((fft.bin_to_frequency(10, 48000) - 937.5).abs() < 0.1);
    }
}
