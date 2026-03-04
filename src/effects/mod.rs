//! LED effect engine
//!
//! Provides various audio-reactive LED effects:
//! - **Energy**: Maps bass/mid/high to RGB
//! - **Spectrum**: Frequency bands across LED strip
//! - **Scroll**: Scrolling color history
//! - **Reactive**: Beat-reactive pulsing
//! - **Pulse**: Beat-synchronized strobe
//! - **VuMeter**: Classic VU meter display

mod traits;
mod energy;
mod spectrum;
mod scroll;
mod reactive;
mod pulse;
mod vumeter;
mod chromafreq;

pub use traits::{Effect, Gradient, Rgb};
pub use energy::EnergyEffect;
pub use spectrum::SpectrumEffect;
pub use scroll::ScrollEffect;
pub use reactive::ReactiveEffect;
pub use pulse::PulseEffect;
pub use vumeter::VuMeterEffect;
pub use chromafreq::ChromaFreqEffect;

use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EffectError {
    #[error("Unknown effect: {0}")]
    UnknownEffect(String),
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
}

/// Effect factory for creating effects by name
pub struct EffectRegistry {
    factories: HashMap<&'static str, Box<dyn Fn(usize) -> Box<dyn Effect> + Send + Sync>>,
}

impl EffectRegistry {
    /// Create a new registry with all built-in effects
    pub fn new() -> Self {
        let mut registry = Self {
            factories: HashMap::new(),
        };

        registry.register("energy", |num_leds| Box::new(EnergyEffect::new(num_leds)));
        registry.register("spectrum", |num_leds| Box::new(SpectrumEffect::new(num_leds)));
        registry.register("scroll", |num_leds| Box::new(ScrollEffect::new(num_leds)));
        registry.register("reactive", |num_leds| Box::new(ReactiveEffect::new(num_leds)));
        registry.register("pulse", |num_leds| Box::new(PulseEffect::new(num_leds)));
        registry.register("vumeter", |num_leds| Box::new(VuMeterEffect::new(num_leds)));
        registry.register("chromafreq", |num_leds| Box::new(ChromaFreqEffect::new(num_leds)));

        registry
    }

    /// Register a new effect factory
    pub fn register<F>(&mut self, name: &'static str, factory: F)
    where
        F: Fn(usize) -> Box<dyn Effect> + Send + Sync + 'static,
    {
        self.factories.insert(name, Box::new(factory));
    }

    /// Create an effect by name
    pub fn create(&self, name: &str, num_leds: usize) -> Result<Box<dyn Effect>, EffectError> {
        self.factories
            .get(name)
            .map(|factory| factory(num_leds))
            .ok_or_else(|| EffectError::UnknownEffect(name.to_string()))
    }

    /// List all available effect names
    pub fn list(&self) -> Vec<&'static str> {
        self.factories.keys().copied().collect()
    }
}

impl Default for EffectRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_list() {
        let registry = EffectRegistry::new();
        let effects = registry.list();

        assert!(effects.contains(&"energy"));
        assert!(effects.contains(&"spectrum"));
        assert!(effects.contains(&"scroll"));
        assert!(effects.contains(&"reactive"));
        assert!(effects.contains(&"pulse"));
        assert!(effects.contains(&"vumeter"));
        assert_eq!(effects.len(), 7);
    }

    #[test]
    fn test_registry_create() {
        let registry = EffectRegistry::new();

        let effect = registry.create("energy", 60);
        assert!(effect.is_ok());
        assert_eq!(effect.unwrap().name(), "energy");

        let effect = registry.create("nonexistent", 60);
        assert!(effect.is_err());
    }

    #[test]
    fn test_energy_effect() {
        let mut effect = EnergyEffect::new(60);
        let mel_bands = vec![0.5; 24];
        let mut output = vec![Rgb::black(); 60];

        effect.render(&mel_bands, None, &mut output);

        // All LEDs should have some color
        assert!(output.iter().all(|led| led.r > 0 || led.g > 0 || led.b > 0));
    }

    #[test]
    fn test_spectrum_effect() {
        let mut effect = SpectrumEffect::new(60);
        let mel_bands = vec![0.5; 24];
        let mut output = vec![Rgb::black(); 60];

        effect.render(&mel_bands, None, &mut output);

        // Some LEDs should have color
        assert!(output.iter().any(|led| led.r > 0 || led.g > 0 || led.b > 0));
    }

    #[test]
    fn test_scroll_effect() {
        let mut effect = ScrollEffect::new(60);
        let mel_bands = vec![0.5; 24];
        let mut output = vec![Rgb::black(); 60];

        // Render multiple frames
        for _ in 0..10 {
            effect.render(&mel_bands, None, &mut output);
        }

        // First LED should have color after scrolling
        assert!(output[0].r > 0 || output[0].g > 0 || output[0].b > 0);
    }

    #[test]
    fn test_reactive_effect_beat() {
        let mut effect = ReactiveEffect::new(60);
        let mel_bands = vec![0.1; 24];
        let mut output = vec![Rgb::black(); 60];

        // Render without beat
        effect.render(&mel_bands, None, &mut output);
        let without_beat = output[0];

        // Render with beat
        effect.render(&mel_bands, Some(1.0), &mut output);
        let with_beat = output[0];

        // Beat should increase brightness
        let brightness_without = without_beat.r as u32 + without_beat.g as u32 + without_beat.b as u32;
        let brightness_with = with_beat.r as u32 + with_beat.g as u32 + with_beat.b as u32;
        assert!(brightness_with >= brightness_without);
    }

    #[test]
    fn test_pulse_effect_beat() {
        let mut effect = PulseEffect::new(60);
        let mel_bands = vec![0.1; 24];
        let mut output = vec![Rgb::black(); 60];

        // Render with beat
        effect.render(&mel_bands, Some(1.0), &mut output);

        // Should have visible output after beat
        assert!(output.iter().any(|led| led.r > 0 || led.g > 0 || led.b > 0));
    }

    #[test]
    fn test_vumeter_effect() {
        let mut effect = VuMeterEffect::new(60);
        let mel_bands = vec![0.5; 24];
        let mut output = vec![Rgb::black(); 60];

        effect.render(&mel_bands, None, &mut output);

        // Should have some lit LEDs
        let lit_count = output.iter().filter(|led| led.r > 0 || led.g > 0 || led.b > 0).count();
        assert!(lit_count > 0);
        assert!(lit_count < 60); // Not all should be lit at 50% level
    }

    #[test]
    fn test_effect_params() {
        let mut effect = EnergyEffect::new(60);

        assert!(effect.set_param("brightness", 0.5).is_ok());
        assert!(effect.set_param("smoothing", 0.8).is_ok());
        assert!(effect.set_param("invalid", 0.5).is_err());
    }

    #[test]
    fn test_gradient_presets() {
        let names = Gradient::list_names();
        assert!(names.contains(&"rainbow"));
        assert!(names.contains(&"fire"));
        assert!(names.contains(&"ocean"));
        assert!(names.contains(&"forest"));
        assert!(names.contains(&"sunset"));
        assert!(names.contains(&"party"));
        assert!(names.contains(&"lava"));

        // All named gradients should work
        for name in names {
            let gradient = Gradient::by_name(name);
            let color = gradient.get(0.5);
            // Should produce valid colors
            assert!(color.r <= 255);
        }
    }
}
