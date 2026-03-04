//! ChromaFreq effect — spectrum spread across LEDs with hue-mapped levels
//!
//! Each LED represents a frequency position. The energy level at that frequency
//! determines the hue (color wheel), and the attack envelope controls brightness.

use super::{Effect, EffectError, Rgb};

pub struct ChromaFreqEffect {
    num_leds: usize,
    /// Per-LED attack/release envelope for brightness
    envelope: Box<[f32]>,
    /// Previous mel band values for attack detection
    prev_bands: Box<[f32]>,
    /// Attack coefficient (fast rise)
    attack: f32,
    /// Release coefficient (slow fade)
    release: f32,
    /// Brightness multiplier
    brightness: f32,
}

impl ChromaFreqEffect {
    pub fn new(num_leds: usize) -> Self {
        Self {
            num_leds,
            envelope: vec![0.0; num_leds].into_boxed_slice(),
            prev_bands: vec![0.0; 64].into_boxed_slice(),
            attack: 0.15,
            release: 0.92,
            brightness: 1.0,
        }
    }

    /// Interpolate mel bands to get energy at a fractional band position
    #[inline]
    fn interpolate_band(mel_bands: &[f32], pos: f32) -> f32 {
        let idx = pos * (mel_bands.len() - 1) as f32;
        let lo = idx as usize;
        let hi = (lo + 1).min(mel_bands.len() - 1);
        let frac = idx - lo as f32;
        mel_bands[lo] * (1.0 - frac) + mel_bands[hi] * frac
    }
}

impl Effect for ChromaFreqEffect {
    fn name(&self) -> &'static str {
        "chromafreq"
    }

    fn render(&mut self, mel_bands: &[f32], _beat: Option<f32>, output: &mut [Rgb]) {
        if mel_bands.is_empty() || output.is_empty() {
            return;
        }

        // Resize prev_bands if needed
        if self.prev_bands.len() != mel_bands.len() {
            self.prev_bands = vec![0.0; mel_bands.len()].into_boxed_slice();
        }

        for (i, led) in output.iter_mut().enumerate().take(self.num_leds) {
            // Map LED position to frequency band (linear spread)
            let pos = i as f32 / self.num_leds as f32;
            let energy = Self::interpolate_band(mel_bands, pos);
            let prev_energy = Self::interpolate_band(&self.prev_bands, pos);

            // Attack detection: how much the signal is rising right now
            let delta = (energy - prev_energy).max(0.0);

            // Update envelope: fast attack, slow release
            let target = (energy + delta * 3.0).clamp(0.0, 1.0);
            if target > self.envelope[i] {
                self.envelope[i] += (target - self.envelope[i]) * (1.0 - self.attack);
            } else {
                self.envelope[i] *= self.release;
            }

            // Hue from energy level: 0.0 (low) → blue(240°), 1.0 (high) → red(0°)
            let hue = (1.0 - energy.clamp(0.0, 1.0)) * 240.0;

            // Brightness from envelope
            let value = self.envelope[i].clamp(0.0, 1.0) * self.brightness;

            *led = Rgb::from_hsv(hue, 1.0, value);
        }

        // Store current bands for next frame's attack detection
        self.prev_bands.copy_from_slice(mel_bands);
    }

    fn set_param(&mut self, name: &str, value: f32) -> Result<(), EffectError> {
        match name {
            "brightness" => {
                self.brightness = value.clamp(0.0, 1.0);
                Ok(())
            }
            "attack" => {
                self.attack = value.clamp(0.0, 0.99);
                Ok(())
            }
            "release" => {
                self.release = value.clamp(0.0, 0.999);
                Ok(())
            }
            _ => Err(EffectError::InvalidParameter(name.to_string())),
        }
    }

    fn reset(&mut self) {
        self.envelope.fill(0.0);
        self.prev_bands.fill(0.0);
    }
}
