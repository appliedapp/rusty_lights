//! Spectrum analyzer effect

use super::{Effect, EffectError, Gradient, Rgb};

/// Spectrum effect that maps frequency bands across the LED strip
pub struct SpectrumEffect {
    num_leds: usize,
    /// Mapping from LED index to mel band index
    led_to_band: Box<[usize]>,
    /// Color gradient
    gradient: Gradient,
    /// Whether to mirror the effect
    mirror: bool,
    /// Brightness multiplier
    brightness: f32,
}

impl SpectrumEffect {
    /// Create a new spectrum effect
    pub fn new(num_leds: usize) -> Self {
        Self {
            num_leds,
            led_to_band: vec![0; num_leds].into_boxed_slice(),
            gradient: Gradient::rainbow(),
            mirror: false,
            brightness: 1.0,
        }
    }

    /// Update LED-to-band mapping based on number of bands
    fn update_mapping(&mut self, num_bands: usize) {
        let effective_leds = if self.mirror {
            self.num_leds / 2
        } else {
            self.num_leds
        };

        for (i, band) in self.led_to_band.iter_mut().enumerate() {
            let led_pos = if self.mirror && i >= effective_leds {
                self.num_leds - 1 - i
            } else {
                i.min(effective_leds - 1)
            };

            *band = (led_pos * num_bands / effective_leds).min(num_bands - 1);
        }
    }

    /// Set the color gradient
    pub fn set_gradient(&mut self, name: &str) {
        self.gradient = match name {
            "fire" => Gradient::fire(),
            "ocean" => Gradient::ocean(),
            _ => Gradient::rainbow(),
        };
    }
}

impl Effect for SpectrumEffect {
    fn name(&self) -> &'static str {
        "spectrum"
    }

    fn render(&mut self, mel_bands: &[f32], _beat: Option<f32>, output: &mut [Rgb]) {
        if mel_bands.is_empty() || output.is_empty() {
            return;
        }

        // Update mapping if bands changed
        if self.led_to_band.iter().any(|&b| b >= mel_bands.len()) {
            self.update_mapping(mel_bands.len());
        }

        for (i, led) in output.iter_mut().enumerate().take(self.num_leds) {
            let band_idx = self.led_to_band[i];
            let energy = mel_bands[band_idx].clamp(0.0, 1.0);

            // Get color from gradient based on LED position
            let pos = i as f32 / self.num_leds as f32;
            let color = self.gradient.get(pos);

            // Scale by energy and brightness
            *led = color.scale(energy * self.brightness);
        }
    }

    fn set_param(&mut self, name: &str, value: f32) -> Result<(), EffectError> {
        match name {
            "brightness" => {
                self.brightness = value.clamp(0.0, 1.0);
                Ok(())
            }
            "mirror" => {
                self.mirror = value > 0.5;
                self.update_mapping(24); // Default bands
                Ok(())
            }
            _ => Err(EffectError::InvalidParameter(name.to_string())),
        }
    }

    fn reset(&mut self) {
        // Nothing to reset
    }
}
