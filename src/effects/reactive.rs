// SPDX-License-Identifier: GPL-3.0-only
// Copyright (c) 2026 appliedappliance GmbH
//! Beat-reactive pulsing effect

use super::{Effect, EffectError, Rgb};

/// Reactive effect that pulses with the beat
pub struct ReactiveEffect {
    num_leds: usize,
    /// Current pulse intensity (decays over time)
    pulse: f32,
    /// Base color hue
    hue: f32,
    /// Pulse decay rate
    decay: f32,
    /// Brightness multiplier
    brightness: f32,
}

impl ReactiveEffect {
    /// Create a new reactive effect
    pub fn new(num_leds: usize) -> Self {
        Self {
            num_leds,
            pulse: 0.0,
            hue: 0.0,
            decay: 0.92,
            brightness: 1.0,
        }
    }
}

impl Effect for ReactiveEffect {
    fn name(&self) -> &'static str {
        "reactive"
    }

    fn render(&mut self, mel_bands: &[f32], beat: Option<f32>, output: &mut [Rgb]) {
        if output.is_empty() {
            return;
        }

        // On beat, boost pulse and shift hue
        if let Some(strength) = beat {
            self.pulse = (self.pulse + strength).min(1.0);
            self.hue = (self.hue + 30.0 * strength) % 360.0;
        }

        // Calculate background intensity from bass
        let bass_energy = if !mel_bands.is_empty() {
            let bass_bands = mel_bands.len() / 4;
            mel_bands[..bass_bands].iter().sum::<f32>() / bass_bands as f32
        } else {
            0.0
        };

        // Combine pulse and bass energy
        let intensity = (self.pulse * 0.7 + bass_energy * 0.3).clamp(0.0, 1.0);

        // Generate color
        let color = Rgb::from_hsv(self.hue, 0.9, intensity * self.brightness);

        // Fill all LEDs
        for led in output.iter_mut().take(self.num_leds) {
            *led = color;
        }

        // Decay pulse
        self.pulse *= self.decay;
    }

    fn set_param(&mut self, name: &str, value: f32) -> Result<(), EffectError> {
        match name {
            "brightness" => {
                self.brightness = value.clamp(0.0, 1.0);
                Ok(())
            }
            "decay" => {
                self.decay = value.clamp(0.8, 0.99);
                Ok(())
            }
            "hue" => {
                self.hue = value % 360.0;
                Ok(())
            }
            _ => Err(EffectError::InvalidParameter(name.to_string())),
        }
    }

    fn reset(&mut self) {
        self.pulse = 0.0;
    }
}
