// SPDX-License-Identifier: GPL-3.0-only
// Copyright (c) 2026 appliedappliance GmbH
//! Beat-synchronized pulse/strobe effect

use super::{Effect, EffectError, Gradient, Rgb};

/// Pulse effect that flashes on beat with configurable decay
///
/// Creates a strobe-like effect synchronized to the beat.
pub struct PulseEffect {
    num_leds: usize,
    /// Current intensity (0.0 - 1.0)
    intensity: f32,
    /// Decay rate per frame
    decay: f32,
    /// Color gradient for intensity mapping
    gradient: Gradient,
    /// Base brightness
    brightness: f32,
    /// Minimum intensity (background glow)
    min_intensity: f32,
    /// Use gradient vs single color
    use_gradient: bool,
    /// Fixed hue when not using gradient
    hue: f32,
}

impl PulseEffect {
    /// Create a new pulse effect
    pub fn new(num_leds: usize) -> Self {
        Self {
            num_leds,
            intensity: 0.0,
            decay: 0.85,
            gradient: Gradient::fire(),
            brightness: 1.0,
            min_intensity: 0.05,
            use_gradient: false,
            hue: 0.0, // Red
        }
    }

    /// Set the color gradient
    pub fn set_gradient(&mut self, name: &str) {
        self.gradient = match name {
            "fire" => Gradient::fire(),
            "ocean" => Gradient::ocean(),
            "rainbow" => Gradient::rainbow(),
            _ => Gradient::fire(),
        };
    }
}

impl Effect for PulseEffect {
    fn name(&self) -> &'static str {
        "pulse"
    }

    fn render(&mut self, mel_bands: &[f32], beat: Option<f32>, output: &mut [Rgb]) {
        if output.is_empty() {
            return;
        }

        // On beat, set intensity to max
        if let Some(strength) = beat {
            self.intensity = (self.intensity + strength * 0.8).min(1.0);
        }

        // Add subtle bass response
        if !mel_bands.is_empty() {
            let bass_bands = (mel_bands.len() / 6).max(1);
            let bass: f32 = mel_bands[..bass_bands].iter().sum::<f32>() / bass_bands as f32;
            self.intensity = (self.intensity + bass * 0.1).min(1.0);
        }

        // Ensure minimum intensity
        let display_intensity = self.intensity.max(self.min_intensity);

        // Generate color
        let color = if self.use_gradient {
            self.gradient.get(display_intensity).scale(self.brightness)
        } else {
            Rgb::from_hsv(self.hue, 0.9, display_intensity * self.brightness)
        };

        // Fill all LEDs
        for led in output.iter_mut().take(self.num_leds) {
            *led = color;
        }

        // Decay intensity
        self.intensity *= self.decay;
    }

    fn set_param(&mut self, name: &str, value: f32) -> Result<(), EffectError> {
        match name {
            "brightness" => {
                self.brightness = value.clamp(0.0, 1.0);
                Ok(())
            }
            "decay" => {
                self.decay = value.clamp(0.7, 0.99);
                Ok(())
            }
            "hue" => {
                self.hue = value % 360.0;
                self.use_gradient = false;
                Ok(())
            }
            "min_intensity" => {
                self.min_intensity = value.clamp(0.0, 0.3);
                Ok(())
            }
            _ => Err(EffectError::InvalidParameter(name.to_string())),
        }
    }

    fn reset(&mut self) {
        self.intensity = 0.0;
    }
}
