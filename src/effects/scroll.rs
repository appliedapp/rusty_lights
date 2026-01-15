//! Scrolling effect

use super::{Effect, EffectError, Gradient, Rgb};

/// Scroll effect that shifts colors along the strip
///
/// New values are generated on one end and scroll to the other.
pub struct ScrollEffect {
    num_leds: usize,
    /// Color buffer (history)
    buffer: Box<[Rgb]>,
    /// Color gradient
    gradient: Gradient,
    /// Scroll speed (pixels per frame)
    speed: usize,
    /// Brightness multiplier
    brightness: f32,
}

impl ScrollEffect {
    /// Create a new scroll effect
    pub fn new(num_leds: usize) -> Self {
        Self {
            num_leds,
            buffer: vec![Rgb::black(); num_leds].into_boxed_slice(),
            gradient: Gradient::rainbow(),
            speed: 1,
            brightness: 1.0,
        }
    }
}

impl Effect for ScrollEffect {
    fn name(&self) -> &'static str {
        "scroll"
    }

    fn render(&mut self, mel_bands: &[f32], beat: Option<f32>, output: &mut [Rgb]) {
        if mel_bands.is_empty() || output.is_empty() {
            return;
        }

        // Scroll existing colors to the right
        for _ in 0..self.speed {
            for i in (1..self.buffer.len()).rev() {
                self.buffer[i] = self.buffer[i - 1];
            }
        }

        // Calculate new color based on overall energy
        let energy: f32 = mel_bands.iter().sum::<f32>() / mel_bands.len() as f32;
        let energy = energy.clamp(0.0, 1.0);

        // Use beat for hue shift if detected
        let hue_shift = beat.map_or(0.0, |b| b * 60.0);

        // Generate new color at position 0
        let base_hue = (energy * 120.0 + hue_shift) % 360.0; // Green-ish for low, red for high
        let saturation = 0.8 + energy * 0.2;
        let value = energy * self.brightness;

        self.buffer[0] = Rgb::from_hsv(base_hue, saturation, value);

        // Copy buffer to output
        for (i, led) in output.iter_mut().enumerate().take(self.num_leds) {
            *led = self.buffer[i];
        }
    }

    fn set_param(&mut self, name: &str, value: f32) -> Result<(), EffectError> {
        match name {
            "brightness" => {
                self.brightness = value.clamp(0.0, 1.0);
                Ok(())
            }
            "speed" => {
                self.speed = (value as usize).clamp(1, 10);
                Ok(())
            }
            _ => Err(EffectError::InvalidParameter(name.to_string())),
        }
    }

    fn reset(&mut self) {
        self.buffer.fill(Rgb::black());
    }
}
