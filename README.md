# RustyLights

[![CI](https://github.com/appliedapp/rusty_lights/actions/workflows/ci.yml/badge.svg)](https://github.com/appliedapp/rusty_lights/actions/workflows/ci.yml)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

Resource-efficient audio-reactive LED visualizer written in Rust. Captures audio, processes it through a DSP pipeline, and outputs to LED strips via E1.31/sACN, DDP, or Art-Net.

Designed to run on a Raspberry Pi with sub-5% CPU usage.

## Features

- **Audio backends**: PipeWire, ALSA, FIFO (for MPD-based players)
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

## Installation

### Quick install (Debian/Raspberry Pi OS)

```bash
curl -sSL https://raw.githubusercontent.com/appliedapp/rusty_lights/main/install.sh | sudo sh
```

This downloads the latest `.deb` package for your architecture, installs the binary, config, and systemd service. Then:

```bash
sudo nano /etc/rusty_lights.conf          # adjust to your setup
sudo systemctl enable --now rusty_lights
```

Pre-built `.deb` packages for amd64, arm64, and armhf are available on the [Releases](https://github.com/appliedapp/rusty_lights/releases) page.

### Manual install

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

## MPD-Based Player Integration

The FIFO backend works with any audio player built on [MPD](https://www.musicpd.org/), including:

- [moOde Audio](https://moodeaudio.org/)
- [Volumio](https://volumio.com/)
- [RuneAudio](https://www.runeaudio.com/)
- [myMPD](https://jcorporation.github.io/myMPD/)
- Plain MPD installations

### Option A: MPD FIFO (MPD playback only)

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

### Option B: ALSA Loopback (all sources, moOde example)

This captures audio from all sources (MPD, Spotify, AirPlay, etc.) by routing through an ALSA loopback device. Shown here for moOde, but the principle applies to any ALSA-based setup.

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

## Architecture

### Thread model (3 threads)

1. **Main** — CLI (clap), config loading, signal handling
2. **Audio** — PipeWire/ALSA/FIFO callback writes to lock-free SPSC ring buffer
3. **Processing** — Reads ring buffer → DSP pipeline → effect render → protocol output

### Data flow per frame

```
Audio callback → RingBuffer(SPSC, 8192 samples)
    → FftProcessor(512-pt, Hanning window)
    → MelBank(24 bands, 20-18kHz)
    → Smoother(EMA) + BeatDetector(spectral flux)
    → Effect.render(mel_bands, beat) → RGB buffer
    → util::reorder_rgb() + gamma correction
    → E131/DDP/ArtNet sender (UDP)
```

### Design constraints

- Zero allocations per frame after initialization — reuse buffers
- Target: <5% CPU on RPi 4, <150µs per frame
- No async runtime — simple threading is sufficient
- Lock-free audio path — no mutexes between audio and processing threads

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
| `idle_timeout` | Close output after N minutes of silence | 0 (disabled) - any value in minutes |

## Roadmap

- **LED layouts** — Support for matrix, ring, and custom LED arrangements with coordinate mapping (serpentine wiring, 2D/polar coordinates). Enables layout-aware effects like matrix equalizers and radial pulses.
- **WLED auto-discovery** — ~~Find WLED controller via mDNS~~, ~~auto-configure LED count~~ and ~~RGB order~~ from the WLED JSON API. ~~Zero-config setup with `target = "auto"`~~. Remaining: layout (strip/matrix) detection, multi-device discovery.
- **Spectrogram effect** — Scrolling frequency-time display across the LED strip using full 257-bin FFT resolution.
- **Multi-band onset effect** — Independent beat detection per frequency region (kick, snare, hi-hat) using per-band spectral flux. Qualitative leap beyond single global beat triggers.
- **Performance optimizations** — ~~Bulk-copy ring buffer~~, eliminate per-frame allocations in DDP sender, remove redundant RGB↔DMX conversion in processing loop
- **Scenes/presets** — Save and recall effect + parameter combinations
- **Multi-device output** — Drive multiple LED controllers simultaneously via segment-to-target mapping. `MultiSegmentOutput` wraps multiple outputs behind the `LedOutput` trait, slicing the LED buffer per segment. Auto-discovery finds all WLEDs on the network and chains their LED counts.

## License

This project is licensed under the [GNU General Public License v3.0](LICENSE).
