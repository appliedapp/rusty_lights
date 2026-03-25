// SPDX-License-Identifier: GPL-3.0-only
// Copyright (c) 2026 appliedappliance GmbH
//! Classic VU meter effect

use super::{Effect, EffectError, Rgb};
use crate::dsp::AttackReleaseSmoother;

/// VU Meter effect that displays audio level as a bar graph
///
/// Classic LED strip visualization with green-yellow-red gradient.
pub struct VuMeterEffect {
    num_leds: usize,
    /// Smoother for level
    smoother: AttackReleaseSmoother,
    /// Peak hold position
    peak_pos: f32,
    /// Peak hold decay
    peak_decay: f32,
    /// Show peak indicator
    show_peak: bool,
    /// Brightness multiplier
    brightness: f32,
    /// Reverse direction
    reverse: bool,
}

impl VuMeterEffect {
    /// Create a new VU meter effect
    pub fn new(num_leds: usize) -> Self {
        Self {
            num_leds,
            smoother: AttackReleaseSmoother::for_visualization(1),
            peak_pos: 0.0,
            peak_decay: 0.98,
            show_peak: true,
            brightness: 1.0,
            reverse: false,
        }
    }

    /// Get color for a given position (0.0 = start, 1.0 = end)
    fn position_color(&self, pos: f32) -> Rgb {
        // Green (0-60%) -> Yellow (60-80%) -> Red (80-100%)
        if pos < 0.6 {
            Rgb::new(0, 255, 0) // Green
        } else if pos < 0.8 {
            let t = (pos - 0.6) / 0.2;
            Rgb::lerp(Rgb::new(0, 255, 0), Rgb::new(255, 255, 0), t)
        } else {
            let t = (pos - 0.8) / 0.2;
            Rgb::lerp(Rgb::new(255, 255, 0), Rgb::new(255, 0, 0), t)
        }
    }
}

impl Effect for VuMeterEffect {
    fn name(&self) -> &'static str {
        "vumeter"
    }

    fn render(&mut self, mel_bands: &[f32], _beat: Option<f32>, output: &mut [Rgb]) {
        if output.is_empty() {
            return;
        }

        // Calculate overall level from mel bands
        let level = if !mel_bands.is_empty() {
            // Weight bass more heavily (typical for VU meters)
            let bass_weight = 1.5;
            let mid_weight = 1.0;
            let high_weight = 0.7;

            let third = mel_bands.len() / 3;
            let bass: f32 = mel_bands[..third].iter().sum::<f32>() / third as f32;
            let mid: f32 = mel_bands[third..third * 2].iter().sum::<f32>() / third as f32;
            let high: f32 =
                mel_bands[third * 2..].iter().sum::<f32>() / (mel_bands.len() - third * 2) as f32;

            let weighted = bass * bass_weight + mid * mid_weight + high * high_weight;
            (weighted / (bass_weight + mid_weight + high_weight)).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Apply smoothing
        let mut smoothed = [level];
        self.smoother.process_inplace(&mut smoothed);
        let display_level = smoothed[0];

        // Update peak
        if display_level > self.peak_pos {
            self.peak_pos = display_level;
        } else {
            self.peak_pos *= self.peak_decay;
        }

        // Calculate how many LEDs to light
        let lit_count = (display_level * self.num_leds as f32) as usize;
        let peak_led = (self.peak_pos * (self.num_leds - 1) as f32) as usize;

        // Render
        for (i, led) in output.iter_mut().enumerate().take(self.num_leds) {
            let idx = if self.reverse {
                self.num_leds - 1 - i
            } else {
                i
            };
            let pos = idx as f32 / self.num_leds as f32;

            if idx < lit_count {
                *led = self.position_color(pos).scale(self.brightness);
            } else if self.show_peak && idx == peak_led {
                // Peak indicator
                *led = self.position_color(pos).scale(self.brightness);
            } else {
                *led = Rgb::black();
            }
        }
    }

    fn set_param(&mut self, name: &str, value: f32) -> Result<(), EffectError> {
        match name {
            "brightness" => {
                self.brightness = value.clamp(0.0, 1.0);
                Ok(())
            }
            "peak_decay" => {
                self.peak_decay = value.clamp(0.9, 0.999);
                Ok(())
            }
            "show_peak" => {
                self.show_peak = value > 0.5;
                Ok(())
            }
            "reverse" => {
                self.reverse = value > 0.5;
                Ok(())
            }
            _ => Err(EffectError::InvalidParameter(name.to_string())),
        }
    }

    fn reset(&mut self) {
        self.smoother.reset();
        self.peak_pos = 0.0;
    }
}
