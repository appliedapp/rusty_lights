# RustyLights

Resource-efficient audio-reactive LED visualizer written in Rust. Captures audio, processes it through a DSP pipeline, and outputs to LED strips via E1.31/sACN, DDP, or Art-Net.

Designed to run on a Raspberry Pi with sub-5% CPU usage.

## Features

- **Audio backends**: PipeWire, ALSA, FIFO (for MPD/moOde integration)
- **DSP pipeline**: FFT, Mel filterbank, attack/release smoothing, AGC, beat detection
- **7 effects**: energy, spectrum, scroll, reactive, pulse, vumeter, chromafreq
- **7 gradients**: rainbow, fire, ocean, forest, sunset, party, lava
- **Output protocols**: E1.31/sACN, DDP (WLED), Art-Net
- **Multi-universe**: auto-splits large LED arrays across universes

## Quick Start

### Build

```bash
# Desktop (PipeWire, default)
cargo build --release

# For systems with ALSA only (e.g. moOde, Raspberry Pi)
cargo build --release --no-default-features --features alsa

# FIFO-only (no audio library dependencies)
cargo build --release --no-default-features
```

### Run

```bash
# Start visualizer (auto-detects /etc/rusty_lights.conf or ./rusty_lights.toml)
rusty_lights run

# Use a specific config file
rusty_lights -c /path/to/config.toml run

# List audio devices
rusty_lights devices

# List available effects
rusty_lights effects

# Send test pattern to LEDs
rusty_lights test

# Verbose logging
rusty_lights run -vv
```

### Configuration

Without `-c`, the config file is resolved in order:

1. `/etc/rusty_lights.conf`
2. `./rusty_lights.toml`

If neither exists, built-in defaults are used.

```bash
# System-wide (recommended for Pi deployments)
sudo cp rusty_lights.example.toml /etc/rusty_lights.conf

# Or local
cp rusty_lights.example.toml rusty_lights.toml
```

```toml
[audio]
backend = "fifo"                # pipewire | alsa | fifo
fifo_path = "/tmp/mpd.fifo"    # for FIFO backend
sample_rate = 48000

[dsp]
smoothing = 0.6                 # 0.0-1.0 (release time, higher = slower fade)
beat_sensitivity = 1.5          # 1.0-3.0 (higher = fewer beats detected)

[effect]
name = "chromafreq"             # energy | spectrum | scroll | reactive | pulse | vumeter | chromafreq

[effect.params]
brightness = 1.0

[output]
protocol = "ddp"                # e131 | ddp | artnet
target = "192.168.1.100"        # IP of LED controller (or multicast for E1.31)
fps = 60

[output.leds]
count = 240
rgb_order = "GRB"               # RGB | GRB | BGR | RBG | BRG | GBR
```

## Cross-Compilation for Raspberry Pi

Cross-compilation uses [cross](https://github.com/cross-rs/cross) which handles all toolchains and libraries via containers. Works with Docker or Podman.

### Setup (once)

```bash
cargo install cross

# If using Podman instead of Docker:
export CROSS_CONTAINER_ENGINE=podman
```

### Raspberry Pi 4 / Pi 3B+ (64-bit, aarch64)

```bash
cross build --release --target aarch64-unknown-linux-gnu --no-default-features --features alsa
scp target/aarch64-unknown-linux-gnu/release/rusty_lights pi@<pi-ip>:~/
```

### Raspberry Pi 3 / Pi 2 / Pi Zero 2 W (32-bit, armv7)

```bash
cross build --release --target armv7-unknown-linux-gnueabihf --no-default-features --features alsa
scp target/armv7-unknown-linux-gnueabihf/release/rusty_lights pi@<pi-ip>:~/
```

### Raspberry Pi Zero / Zero W (armv6)

```bash
cross build --release --target arm-unknown-linux-gnueabihf --no-default-features --features alsa
scp target/arm-unknown-linux-gnueabihf/release/rusty_lights pi@<pi-ip>:~/
```

### Size-optimized build (for constrained devices)

Add `--profile release-small` to any of the above commands for a smaller binary (uses `opt-level = "s"` and fat LTO).

## Running as systemd Service

Install the binary and config on your Pi, then create a service unit:

```bash
sudo cp rusty_lights /usr/local/bin/
sudo cp rusty_lights.example.toml /etc/rusty_lights.conf
# Edit /etc/rusty_lights.conf to match your setup
```

Create `/etc/systemd/system/rusty_lights.service`:

```ini
[Unit]
Description=RustyLights audio-reactive LED visualizer
After=network.target sound.target

[Service]
ExecStart=/usr/local/bin/rusty_lights run
Restart=on-failure
RestartSec=3

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable rusty_lights
sudo systemctl start rusty_lights

# Check status / logs
sudo systemctl status rusty_lights
journalctl -u rusty_lights -f
```

## moOde Audio Setup

RustyLights works with [moOde Audio](https://moodeaudio.org/) for audio-reactive LED visualization from any source (MPD, Spotify Connect, AirPlay).

### Option A: MPD FIFO (MPD only)

Add to `/etc/mpd.conf`:

```
audio_output {
    type "fifo"
    name "Visualizer"
    path "/tmp/mpd.fifo"
    format "48000:16:2"
}
```

Restart MPD: `sudo systemctl restart mpd`

Config:
```toml
[audio]
backend = "fifo"
fifo_path = "/tmp/mpd.fifo"
sample_rate = 48000
```

### Option B: ALSA Loopback (all sources)

This captures audio from all sources (MPD, Spotify, AirPlay, etc.) by routing through an ALSA loopback device.

1. Load the loopback module permanently:
   ```bash
   echo "snd-aloop" | sudo tee -a /etc/modules
   sudo modprobe snd-aloop
   ```

2. Enable **ALSA Loopback** in the moOde Web-UI under **System Config**.

3. Config:
   ```toml
   [audio]
   backend = "alsa"
   device = "hw:Loopback,1"
   sample_rate = 48000
   ```

Note: The ALSA backend requires the binary to be built with `--features alsa`.

## Effects

| Effect | Description |
|--------|-------------|
| `energy` | Maps bass/mid/high energy to RGB across all LEDs |
| `spectrum` | Frequency bands mapped to LED positions, gradient-colored |
| `scroll` | Scrolling color history |
| `reactive` | Beat-triggered flash and decay |
| `pulse` | Beat-synchronized pulsing |
| `vumeter` | Classic VU meter display |
| `chromafreq` | Spectrum across LEDs, energy mapped to hue, attack controls brightness |

## DSP Tuning

| Parameter | Effect | Range |
|-----------|--------|-------|
| `smoothing` | LED fade-out speed (release time) | 0.0 (instant) - 1.0 (very slow) |
| `beat_sensitivity` | Beat detection threshold | 1.0 (very sensitive) - 3.0 (only strong beats) |
| `brightness` | Overall LED brightness | 0.0 - 1.0 |

## License

MIT
