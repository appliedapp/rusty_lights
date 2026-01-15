//! Signal smoothing and filtering

/// Exponential Moving Average smoother
///
/// Provides temporal smoothing with configurable attack/release characteristics.
pub struct Smoother {
    /// Smoothing factor (0.0 = no smoothing, 1.0 = maximum smoothing)
    alpha: f32,
    /// Current smoothed state
    state: Box<[f32]>,
}

impl Smoother {
    /// Create a new smoother
    ///
    /// # Arguments
    /// * `size` - Number of channels to smooth
    /// * `smoothing` - Smoothing factor (0.0 - 1.0)
    pub fn new(size: usize, smoothing: f32) -> Self {
        Self {
            alpha: smoothing.clamp(0.0, 0.99),
            state: vec![0.0; size].into_boxed_slice(),
        }
    }

    /// Process input samples with smoothing
    ///
    /// output[i] = alpha * state[i] + (1 - alpha) * input[i]
    #[inline]
    pub fn process(&mut self, input: &[f32], output: &mut [f32]) {
        let one_minus_alpha = 1.0 - self.alpha;

        for (i, &x) in input.iter().enumerate() {
            if i < self.state.len() {
                self.state[i] = self.alpha * self.state[i] + one_minus_alpha * x;
                if i < output.len() {
                    output[i] = self.state[i];
                }
            }
        }
    }

    /// Process in-place
    #[inline]
    pub fn process_inplace(&mut self, data: &mut [f32]) {
        let one_minus_alpha = 1.0 - self.alpha;

        for (i, x) in data.iter_mut().enumerate() {
            if i < self.state.len() {
                self.state[i] = self.alpha * self.state[i] + one_minus_alpha * *x;
                *x = self.state[i];
            }
        }
    }

    /// Set smoothing factor
    pub fn set_smoothing(&mut self, smoothing: f32) {
        self.alpha = smoothing.clamp(0.0, 0.99);
    }

    /// Reset state to zero
    pub fn reset(&mut self) {
        self.state.fill(0.0);
    }
}

/// Automatic Gain Control
///
/// Normalizes signal levels over time for consistent output.
pub struct Agc {
    /// Current gain level
    gain: f32,
    /// Target peak level
    target: f32,
    /// Attack time constant
    attack: f32,
    /// Release time constant
    release: f32,
    /// Minimum gain
    min_gain: f32,
    /// Maximum gain
    max_gain: f32,
}

impl Agc {
    /// Create a new AGC
    ///
    /// # Arguments
    /// * `target` - Target peak level (0.0 - 1.0)
    /// * `attack` - Attack time constant (smaller = faster)
    /// * `release` - Release time constant (smaller = faster)
    pub fn new(target: f32, attack: f32, release: f32) -> Self {
        Self {
            gain: 1.0,
            target,
            attack: attack.clamp(0.001, 0.5),
            release: release.clamp(0.001, 0.5),
            min_gain: 0.1,
            max_gain: 10.0,
        }
    }

    /// Process samples with AGC
    pub fn process(&mut self, input: &[f32], output: &mut [f32]) {
        // Find peak in input
        let peak = input.iter().fold(0.0_f32, |max, &x| max.max(x.abs()));

        if peak > 0.001 {
            let desired_gain = self.target / peak;

            // Apply attack/release envelope
            let alpha = if desired_gain < self.gain {
                self.attack
            } else {
                self.release
            };

            self.gain = self.gain * (1.0 - alpha) + desired_gain * alpha;
            self.gain = self.gain.clamp(self.min_gain, self.max_gain);
        }

        // Apply gain
        for (i, &x) in input.iter().enumerate() {
            if i < output.len() {
                output[i] = (x * self.gain).clamp(0.0, 1.0);
            }
        }
    }

    /// Get current gain level
    #[inline]
    pub fn current_gain(&self) -> f32 {
        self.gain
    }

    /// Reset AGC state
    pub fn reset(&mut self) {
        self.gain = 1.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoother() {
        let mut smoother = Smoother::new(4, 0.5);
        let input = [1.0, 1.0, 1.0, 1.0];
        let mut output = [0.0; 4];

        // First pass
        smoother.process(&input, &mut output);
        assert!(output.iter().all(|&x| x > 0.0 && x < 1.0));

        // After many passes, should converge to input
        for _ in 0..100 {
            smoother.process(&input, &mut output);
        }
        assert!(output.iter().all(|&x| (x - 1.0).abs() < 0.01));
    }

    #[test]
    fn test_agc() {
        let mut agc = Agc::new(0.8, 0.1, 0.05);

        // Low input should be boosted
        let input = [0.1, 0.1, 0.1, 0.1];
        let mut output = [0.0; 4];

        for _ in 0..50 {
            agc.process(&input, &mut output);
        }

        // Output should be higher than input
        assert!(output[0] > input[0]);
    }
}
