// SPDX-License-Identifier: GPL-3.0-only
// Copyright (c) 2026 appliedappliance GmbH
//! RustyLights - Audio-reactive LED controller

use clap::{Parser, Subcommand};
use rusty_lights::config::Config;
use rusty_lights::effects::EffectRegistry;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "rusty_lights")]
#[command(author, version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH"), ")"), about = "Audio-reactive LED controller", long_about = None)]
struct Cli {
    /// Configuration file path [default: /etc/rusty_lights.conf or ./rusty_lights.toml]
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Audio device (overrides config)
    #[arg(short, long)]
    device: Option<String>,

    /// Effect name (overrides config)
    #[arg(short, long)]
    effect: Option<String>,

    /// Increase logging verbosity
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Suppress output
    #[arg(short, long)]
    quiet: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Clone)]
enum Commands {
    /// Start the visualizer (default)
    Run,
    /// List available audio devices
    Devices,
    /// List available effects
    Effects,
    /// List available color gradients
    Gradients,
    /// Show current configuration
    Config,
    /// Send test pattern to LEDs
    Test {
        /// Number of LEDs to test
        #[arg(short, long, default_value = "60")]
        leds: usize,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.quiet {
        log::LevelFilter::Error
    } else {
        match cli.verbose {
            0 => log::LevelFilter::Info,
            1 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        }
    };

    env_logger::Builder::new()
        .filter_level(log_level)
        .format_timestamp_millis()
        .init();

    // Execute command
    match cli.command.clone().unwrap_or(Commands::Run) {
        Commands::Run => run_visualizer(&cli),
        Commands::Devices => list_devices(),
        Commands::Effects => list_effects(),
        Commands::Gradients => list_gradients(),
        Commands::Config => show_config(&cli),
        Commands::Test { leds } => run_test_pattern(leds, &cli),
    }
}

/// Resolve the config file path using a fallback chain:
/// 1. Explicit CLI path (`-c`)
/// 2. `/etc/rusty_lights.conf`
/// 3. `./rusty_lights.toml`
fn resolve_config(cli_path: Option<&Path>) -> PathBuf {
    if let Some(p) = cli_path {
        return p.to_path_buf();
    }
    let etc = PathBuf::from("/etc/rusty_lights.conf");
    if etc.exists() {
        return etc;
    }
    PathBuf::from("rusty_lights.toml")
}

fn run_visualizer(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    use rusty_lights::Engine;
    use std::sync::atomic::Ordering;

    log::info!("Starting RustyLights visualizer...");

    // Load configuration
    let config_path = resolve_config(cli.config.as_deref());
    let mut config = if config_path.exists() {
        log::info!("Loading config from {:?}", config_path);
        Config::load(&config_path)?
    } else {
        log::info!("No config file found, using defaults");
        Config::default()
    };

    // Apply CLI overrides
    if let Some(ref device) = cli.device {
        config.audio.device = device.clone();
    }
    if let Some(ref effect) = cli.effect {
        config.effect.name = effect.clone();
    }

    // WLED auto-discovery and config detection
    #[cfg(feature = "wled")]
    if config.output.target.eq_ignore_ascii_case("auto") {
        if let Some(ip) = rusty_lights::wled::discover_wled() {
            config.output.target = ip;
            if config.output.protocol == "e131" {
                log::info!("Switching protocol to DDP for discovered WLED device");
                config.output.protocol = "ddp".to_string();
            }
            rusty_lights::wled::apply_wled_config(&config.output.target, &mut config.output.leds);
        } else {
            log::error!("No WLED device found on the network");
            return Err("WLED auto-discovery failed".into());
        }
    } else if config.output.protocol == "ddp" {
        rusty_lights::wled::apply_wled_config(&config.output.target, &mut config.output.leds);
    }

    log::info!("Audio: {} ({})", config.audio.backend, config.audio.device);
    log::info!(
        "DSP: FFT={}, Mel bands={}",
        config.dsp.fft_size,
        config.dsp.mel_bands
    );
    log::info!("Effect: {}", config.effect.name);
    log::info!(
        "Output: {} to {} ({} LEDs @ {} FPS)",
        config.output.protocol,
        config.output.target,
        config.output.leds.led_count(),
        config.output.fps
    );

    #[cfg(feature = "http")]
    if config.http.enabled {
        log::info!("Web UI: http://localhost:{}", config.http.port);
    }

    // Create and start engine
    let mut engine = Engine::new(config)?;
    let running = engine.running_flag();

    // Set up Ctrl+C handler
    let r = running.clone();
    ctrlc::set_handler(move || {
        log::info!("Shutdown signal received...");
        r.store(false, Ordering::SeqCst);
    })?;

    engine.start()?;

    log::info!("Visualizer running. Press Ctrl+C to stop.");

    // Wait for shutdown
    while running.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    engine.stop()?;

    log::info!("Shutdown complete.");
    Ok(())
}

fn list_devices() -> Result<(), Box<dyn std::error::Error>> {
    use rusty_lights::audio;

    println!(
        "Available audio backends: {:?}",
        audio::available_backends()
    );
    println!();

    let devices = audio::list_all_devices();

    if devices.is_empty() {
        println!("No audio devices found.");
        println!("Make sure PipeWire or ALSA is running and accessible.");
    } else {
        println!("Available audio devices:");
        for dev in devices {
            println!("  [{}] {} - {}", dev.backend, dev.name, dev.description);
        }
    }

    println!();
    println!("Use -d/--device to specify a device name.");
    Ok(())
}

fn list_effects() -> Result<(), Box<dyn std::error::Error>> {
    let registry = EffectRegistry::new();

    println!("Available effects:");
    for name in registry.list() {
        println!("  - {}", name);
    }
    println!();
    println!("Use -e/--effect to select an effect.");
    Ok(())
}

fn list_gradients() -> Result<(), Box<dyn std::error::Error>> {
    use rusty_lights::effects::Gradient;

    println!("Available gradients:");
    for name in Gradient::list_names() {
        println!("  - {}", name);
    }
    Ok(())
}

fn show_config(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    use rusty_lights::output::available_protocols;

    let config_path = resolve_config(cli.config.as_deref());
    let config = if config_path.exists() {
        println!("Configuration file: {:?}", config_path);
        Config::load(&config_path)?
    } else {
        println!("Configuration file: (using defaults)");
        Config::default()
    };

    println!();
    println!("[audio]");
    println!("  backend = \"{}\"", config.audio.backend);
    println!("  device = \"{}\"", config.audio.device);
    println!("  sample_rate = {}", config.audio.sample_rate);
    println!("  chunk_size = {}", config.audio.chunk_size);

    println!();
    println!("[dsp]");
    println!("  fft_size = {}", config.dsp.fft_size);
    println!("  mel_bands = {}", config.dsp.mel_bands);
    println!("  freq_min = {}", config.dsp.freq_min);
    println!("  freq_max = {}", config.dsp.freq_max);
    println!("  smoothing = {}", config.dsp.smoothing);

    println!();
    println!("[effect]");
    println!("  name = \"{}\"", config.effect.name);

    println!();
    println!("[output]");
    println!("  protocol = \"{}\"", config.output.protocol);
    println!("  target = \"{}\"", config.output.target);
    println!("  universe = {}", config.output.universe);
    println!("  fps = {}", config.output.fps);

    println!();
    println!("[output.leds]");
    println!("  count = {}", config.output.leds.led_count());
    println!("  rgb_order = \"{}\"", config.output.leds.order());

    println!();
    println!("Available protocols: {:?}", available_protocols());

    Ok(())
}

fn run_test_pattern(num_leds: usize, cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    use rusty_lights::effects::Rgb;
    use rusty_lights::output::{RateLimiter, create_output};

    log::info!("Sending test pattern to {} LEDs...", num_leds);

    // Load config for target address
    let config_path = resolve_config(cli.config.as_deref());
    let config = if config_path.exists() {
        Config::load(&config_path)?
    } else {
        Config::default()
    };

    let mut output = create_output(
        &config.output.protocol,
        &config.output.target,
        config.output.universe,
    )?;

    let mut rate_limiter = RateLimiter::new(60);

    // Create rainbow test pattern
    let mut leds: Vec<Rgb> = (0..num_leds)
        .map(|i| Rgb::from_hsv(i as f32 * 360.0 / num_leds as f32, 1.0, 0.5))
        .collect();

    log::info!(
        "Sending to {} via {}",
        config.output.target,
        config.output.protocol
    );

    // Animate for 3 seconds (180 frames at 60fps)
    for frame in 0..180 {
        // Rotate colors
        let hue_offset = frame as f32 * 2.0;
        for (i, led) in leds.iter_mut().enumerate() {
            let hue = (i as f32 * 360.0 / num_leds as f32 + hue_offset) % 360.0;
            *led = Rgb::from_hsv(hue, 1.0, 0.5);
        }

        output.send(&leds)?;
        rate_limiter.wait();
    }

    // Turn off LEDs
    leds.fill(Rgb::black());
    output.send(&leds)?;

    log::info!("Test pattern complete.");
    Ok(())
}
