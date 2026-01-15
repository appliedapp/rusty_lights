//! RustyLights - Audio-reactive LED controller

use clap::{Parser, Subcommand};
use rusty_lights::config::Config;
use rusty_lights::effects::EffectRegistry;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rusty_lights")]
#[command(author, version, about = "Audio-reactive LED controller", long_about = None)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "rusty_lights.toml")]
    config: PathBuf,

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
        Commands::Test { leds } => run_test_pattern(leds, &cli),
    }
}

fn run_visualizer(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Starting RustyLights visualizer...");

    // Load configuration
    let config = if cli.config.exists() {
        log::info!("Loading config from {:?}", cli.config);
        Config::load(&cli.config)?
    } else {
        log::info!("Using default configuration");
        Config::default()
    };

    log::debug!("Audio backend: {}", config.audio.backend);
    log::debug!("FFT size: {}", config.dsp.fft_size);
    log::debug!("Effect: {}", config.effect.name);
    log::debug!("Output: {} to {}", config.output.protocol, config.output.target);

    // TODO: Initialize audio capture
    // TODO: Initialize DSP pipeline
    // TODO: Initialize effect engine
    // TODO: Initialize output
    // TODO: Start main loop

    log::info!("Visualizer ready. Press Ctrl+C to stop.");

    // Wait for shutdown signal
    #[cfg(unix)]
    {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();

        ctrlc::set_handler(move || {
            r.store(false, Ordering::SeqCst);
        })?;

        while running.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    log::info!("Shutting down...");
    Ok(())
}

fn list_devices() -> Result<(), Box<dyn std::error::Error>> {
    use rusty_lights::audio;

    println!("Available audio backends: {:?}", audio::available_backends());
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
    Ok(())
}

fn run_test_pattern(num_leds: usize, cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    use rusty_lights::effects::Rgb;
    use rusty_lights::output::{E131Sender, LedOutput};

    log::info!("Sending test pattern to {} LEDs...", num_leds);

    // Load config for target address
    let config = if cli.config.exists() {
        Config::load(&cli.config)?
    } else {
        Config::default()
    };

    let mut sender = E131Sender::new(&config.output.target, config.output.universe)?;

    // Create rainbow test pattern
    let mut leds: Vec<Rgb> = (0..num_leds)
        .map(|i| Rgb::from_hsv(i as f32 * 360.0 / num_leds as f32, 1.0, 0.5))
        .collect();

    // Animate for a few seconds
    for frame in 0..180 {
        // Rotate colors
        let hue_offset = frame as f32 * 2.0;
        for (i, led) in leds.iter_mut().enumerate() {
            let hue = (i as f32 * 360.0 / num_leds as f32 + hue_offset) % 360.0;
            *led = Rgb::from_hsv(hue, 1.0, 0.5);
        }

        sender.send(&leds)?;
        std::thread::sleep(std::time::Duration::from_millis(16)); // ~60fps
    }

    // Turn off LEDs
    leds.fill(Rgb::black());
    sender.send(&leds)?;

    log::info!("Test pattern complete.");
    Ok(())
}

// Signal handling support
#[cfg(unix)]
extern crate ctrlc;
