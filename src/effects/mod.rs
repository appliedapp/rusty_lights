//! LED effect engine

mod traits;
mod energy;
mod spectrum;
mod scroll;
mod reactive;

pub use traits::{Effect, Gradient, Rgb};
pub use energy::EnergyEffect;
pub use spectrum::SpectrumEffect;
pub use scroll::ScrollEffect;
pub use reactive::ReactiveEffect;

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
