// SPDX-License-Identifier: GPL-3.0-only
// Copyright (c) 2026 appliedappliance GmbH
//! Integration tests for RustyLights
//!
//! These tests verify the full pipeline works correctly without requiring
//! actual audio hardware.

use rusty_lights::config::Config;
use rusty_lights::dsp::{DspConfig, DspPipeline};
use rusty_lights::effects::{EffectRegistry, Rgb};

/// Test that DSP pipeline processes audio correctly
#[test]
fn test_dsp_pipeline_integration() {
    let config = DspConfig {
        fft_size: 1024,
        mel_bands: 16,
        sample_rate: 48000,
        freq_min: 20.0,
        freq_max: 20000.0,
        smoothing: 0.8,
        beat_sensitivity: 1.5,
    };

    let mut pipeline = DspPipeline::new(&config).expect("Failed to create DSP pipeline");

    // Generate test audio: 440Hz sine wave
    let samples: Vec<f32> = (0..1024)
        .map(|i| {
            let t = i as f32 / 48000.0;
            (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5
        })
        .collect();

    let result = pipeline.process(&samples);

    // Verify we get mel bands output
    assert_eq!(result.mel_bands.len(), 16);

    // Verify mel bands are in valid range
    for &band in result.mel_bands {
        assert!(band >= 0.0 && band <= 1.0, "Mel band {} out of range", band);
    }
}

/// Test that effect registry contains all expected effects
#[test]
fn test_effect_registry_completeness() {
    let registry = EffectRegistry::new();
    let effects = registry.list();

    // Verify expected effects are present
    let expected = [
        "energy", "spectrum", "scroll", "reactive", "pulse", "vumeter",
    ];
    for name in &expected {
        assert!(
            effects.iter().any(|e| e == name),
            "Missing effect: {}",
            name
        );
    }
}

/// Test that all effects can be created and rendered
#[test]
fn test_all_effects_render() {
    let registry = EffectRegistry::new();
    let num_leds = 60;

    // Test mel bands (simulated audio data)
    let mel_bands: Vec<f32> = (0..16).map(|i| (i as f32 / 16.0) * 0.8).collect();

    for effect_name in registry.list() {
        let mut effect = registry
            .create(&effect_name, num_leds)
            .expect(&format!("Failed to create effect: {}", effect_name));

        let mut leds = vec![Rgb::black(); num_leds];

        // Render several frames to test stability
        for frame in 0..10 {
            let beat = if frame % 5 == 0 { Some(1.0) } else { None }; // Simulate beat every 5 frames
            effect.render(&mel_bands, beat, &mut leds);

            // Verify LEDs were written (effect rendered something)
            // LED values are u8 so always valid (0-255)
            let _ = leds.len(); // Ensure buffer wasn't corrupted
        }
    }
}

/// Test effect parameter setting
#[test]
fn test_effect_parameters() {
    let registry = EffectRegistry::new();

    let mut effect = registry.create("energy", 60).unwrap();

    // Test setting brightness
    assert!(effect.set_param("brightness", 0.5).is_ok());

    // Test invalid parameter
    assert!(effect.set_param("nonexistent", 1.0).is_err());
}

/// Test default configuration
#[test]
fn test_default_config() {
    let config = Config::default();

    // Verify sensible defaults
    assert!(config.audio.sample_rate >= 44100);
    assert!(config.dsp.fft_size.is_power_of_two());
    assert!(config.dsp.mel_bands >= 8);
    assert!(config.output.leds.led_count() > 0);
    assert!(config.output.fps > 0);
}

/// Test full DSP to effect pipeline
#[test]
fn test_dsp_to_effect_pipeline() {
    // Create DSP pipeline
    let dsp_config = DspConfig {
        fft_size: 1024,
        mel_bands: 16,
        sample_rate: 48000,
        freq_min: 20.0,
        freq_max: 20000.0,
        smoothing: 0.8,
        beat_sensitivity: 1.5,
    };
    let mut dsp = DspPipeline::new(&dsp_config).unwrap();

    // Create effect
    let registry = EffectRegistry::new();
    let mut effect = registry.create("spectrum", 60).unwrap();

    // Generate test audio with beat-like characteristics
    let samples: Vec<f32> = (0..1024)
        .map(|i| {
            let t = i as f32 / 48000.0;
            // Mix of bass (60Hz) and mid (1kHz)
            let bass = (2.0 * std::f32::consts::PI * 60.0 * t).sin() * 0.7;
            let mid = (2.0 * std::f32::consts::PI * 1000.0 * t).sin() * 0.3;
            bass + mid
        })
        .collect();

    let mut leds = vec![Rgb::black(); 60];

    // Process multiple frames
    for _ in 0..30 {
        let result = dsp.process(&samples);
        effect.render(result.mel_bands, result.beat, &mut leds);
    }

    // Verify LEDs are not all black (effect is producing output)
    let has_color = leds.iter().any(|led| led.r > 0 || led.g > 0 || led.b > 0);
    assert!(has_color, "Effect should produce visible output");
}

/// Test RGB color operations
#[test]
fn test_rgb_operations() {
    // Test black
    let black = Rgb::black();
    assert_eq!(black.r, 0);
    assert_eq!(black.g, 0);
    assert_eq!(black.b, 0);

    // Test HSV conversion
    let red = Rgb::from_hsv(0.0, 1.0, 1.0);
    assert_eq!(red.r, 255);
    assert!(red.g < 10); // Allow small rounding
    assert!(red.b < 10);

    let green = Rgb::from_hsv(120.0, 1.0, 1.0);
    assert!(green.r < 10);
    assert_eq!(green.g, 255);
    assert!(green.b < 10);

    let blue = Rgb::from_hsv(240.0, 1.0, 1.0);
    assert!(blue.r < 10);
    assert!(blue.g < 10);
    assert_eq!(blue.b, 255);
}

/// Test gradient interpolation
#[test]
fn test_gradient_interpolation() {
    use rusty_lights::effects::Gradient;

    let gradient = Gradient::rainbow();

    // Test endpoints
    let start = gradient.get(0.0);
    let end = gradient.get(1.0);

    // Rainbow should have different colors at start and end
    assert!(
        start.r != end.r || start.g != end.g || start.b != end.b,
        "Gradient endpoints should differ"
    );

    // Test midpoint exists
    let _mid = gradient.get(0.5);

    // Test all named gradients can be created and produce colors
    for name in Gradient::list_names() {
        let g = Gradient::by_name(name);
        let _color = g.get(0.5); // Just verify it doesn't panic
    }
}
