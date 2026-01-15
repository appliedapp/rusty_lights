//! Energy-based effect (bass=red, mid=green, high=blue)

use super::{Effect, EffectError, Rgb};
use crate::dsp::Smoother;

/// Energy effect that maps frequency bands to RGB colors
///
/// Bass frequencies control red, mids control green, highs control blue.
pub struct EnergyEffect {
    num_leds: usize,
    smoother: Smoother,
    brightness: f32,
}

impl EnergyEffect {
    /// Create a new energy effect
    pub fn new(num_leds: usize) -> Self {
        Self {
            num_leds,
            smoother: Smoother::new(3, 0.7),
            brightness: 1.0,
        }
    }
}

impl Effect for EnergyEffect {
    fn name(&self) -> &'static str {
        "energy"
    }

    fn render(&mut self, mel_bands: &[f32], _beat: Option<f32>, output: &mut [Rgb]) {
        if mel_bands.is_empty() || output.is_empty() {
            return;
        }

        // Divide mel bands into 3 regions: bass, mid, high
        let bands_per_region = mel_bands.len() / 3;

        let bass: f32 = mel_bands[..bands_per_region]
            .iter()
            .sum::<f32>()
            / bands_per_region as f32;

        let mid: f32 = mel_bands[bands_per_region..bands_per_region * 2]
            .iter()
            .sum::<f32>()
            / bands_per_region as f32;

        let high: f32 = mel_bands[bands_per_region * 2..]
            .iter()
            .sum::<f32>()
            / (mel_bands.len() - bands_per_region * 2) as f32;

        // Apply smoothing
        let mut values = [bass, mid, high];
        self.smoother.process_inplace(&mut values);

        // Scale to 0-255 range with brightness
        let r = (values[0] * 255.0 * self.brightness).min(255.0) as u8;
        let g = (values[1] * 255.0 * self.brightness).min(255.0) as u8;
        let b = (values[2] * 255.0 * self.brightness).min(255.0) as u8;

        let color = Rgb::new(r, g, b);

        // Fill all LEDs with the same color
        for led in output.iter_mut().take(self.num_leds) {
            *led = color;
        }
    }

    fn set_param(&mut self, name: &str, value: f32) -> Result<(), EffectError> {
        match name {
            "brightness" => {
                self.brightness = value.clamp(0.0, 1.0);
                Ok(())
            }
            "smoothing" => {
                self.smoother.set_smoothing(value);
                Ok(())
            }
            _ => Err(EffectError::InvalidParameter(name.to_string())),
        }
    }

    fn reset(&mut self) {
        self.smoother.reset();
    }
}
