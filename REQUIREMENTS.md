# RustLED - Work Breakdown Structure

**Projekt:** Ressourceneffiziente Audio-zu-LED-Visualisierung in Rust  
**Zielplattform:** Linux (PipeWire), ARM64/ARMv7/x86_64  
**Protokoll:** E1.31 (sACN), DDP  
**Codename:** `rustled`

---

## Inhaltsverzeichnis

1. [Projektübersicht](#1-projektübersicht)
2. [Architektur](#2-architektur)
3. [Work Packages](#3-work-packages)
4. [Abhängigkeiten & Crates](#4-abhängigkeiten--crates)
5. [Ressourcenoptimierung](#5-ressourcenoptimierung)
6. [Build & Cross-Compilation](#6-build--cross-compilation)
7. [Zeitschätzung](#7-zeitschätzung)
8. [Risiken & Mitigationen](#8-risiken--mitigationen)

---

## 1. Projektübersicht

### 1.1 Ziele

- Echtzeit-Audio-Analyse mit minimaler CPU-Last (<5% auf RPi 4, <15% auf RPi 3)
- E1.31/sACN Output für professionelle LED-Controller und WLED
- Native PipeWire-Integration ohne PulseAudio-Kompatibilitätsschicht
- Single-Binary Deployment ohne Runtime-Abhängigkeiten
- Support für 32-bit ARMv7 (Raspberry Pi 2/3 mit 32-bit OS)

### 1.2 Nicht-Ziele (v1.0)

- Web-UI (später via separatem Frontend)
- Windows/macOS Support
- Bluetooth-Audio-Capture
- Video-Synchronisation

### 1.3 Target Platforms

| Platform | Arch | Target Triple | Min. Hardware |
|----------|------|---------------|---------------|
| RPi 5 | ARM64 | `aarch64-unknown-linux-gnu` | 4GB RAM |
| RPi 4 | ARM64 | `aarch64-unknown-linux-gnu` | 2GB RAM |
| RPi 3/4 32-bit | ARMv7 | `armv7-unknown-linux-gnueabihf` | 1GB RAM |
| RPi 2 | ARMv7 | `armv7-unknown-linux-gnueabihf` | 1GB RAM |
| RPi Zero 2 W | ARM64 | `aarch64-unknown-linux-gnu` | 512MB RAM |
| Generic x86_64 | x86_64 | `x86_64-unknown-linux-gnu` | 512MB RAM |

---

## 2. Architektur

### 2.1 High-Level Datenfluss

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  PipeWire   │───▶│   Audio     │───▶│   Effect    │───▶│   E1.31     │
│  Capture    │    │   DSP       │    │   Engine    │    │   Output    │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
      │                  │                  │                  │
      ▼                  ▼                  ▼                  ▼
   512 samples      FFT Bins +         RGB Array          UDP Packets
   @ 48kHz         Mel Bands          [u8; N*3]          multicast
```

### 2.2 Thread-Modell

```
┌─────────────────────────────────────────────────────────────────┐
│                        Main Thread                               │
│  - Config Loading                                                │
│  - Signal Handling (SIGTERM, SIGHUP)                            │
│  - Lifecycle Management                                          │
└─────────────────────────────────────────────────────────────────┘
                              │
           ┌──────────────────┼──────────────────┐
           ▼                  ▼                  ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│  Audio Thread   │  │  DSP Thread     │  │  Output Thread  │
│  (RT priority)  │  │  (normal)       │  │  (normal)       │
│                 │  │                 │  │                 │
│  - PipeWire     │  │  - FFT          │  │  - E1.31 Tx     │
│    callback     │  │  - Mel Bank     │  │  - Rate Limit   │
│  - Ring Buffer  │  │  - Effects      │  │  - Multicast    │
│    Producer     │  │                 │  │                 │
└────────┬────────┘  └────────┬────────┘  └────────┬────────┘
         │                    │                    │
         └─────► SPSC Queue ──┴─────► SPSC Queue ──┘
              (lock-free)         (lock-free)
```

### 2.3 Modul-Struktur

```
rustled/
├── Cargo.toml
├── src/
│   ├── main.rs                 # Entry, CLI, Signal Handler
│   ├── lib.rs                  # Public API für Tests
│   ├── config/
│   │   ├── mod.rs
│   │   └── schema.rs           # Serde-Strukturen
│   ├── audio/
│   │   ├── mod.rs
│   │   ├── pipewire.rs         # PipeWire Backend
│   │   ├── alsa.rs             # ALSA Fallback (optional)
│   │   └── ringbuffer.rs       # Lock-free Ring Buffer
│   ├── dsp/
│   │   ├── mod.rs
│   │   ├── fft.rs              # FFT Wrapper
│   │   ├── mel.rs              # Mel Filterbank
│   │   ├── filters.rs          # IIR/Smoothing
│   │   └── beat.rs             # Beat Detection
│   ├── effects/
│   │   ├── mod.rs
│   │   ├── traits.rs           # Effect Trait
│   │   ├── energy.rs           # Basis-Effekt
│   │   ├── spectrum.rs         # Spektrum-Visualisierung
│   │   ├── scroll.rs           # Scroll-Effekt
│   │   └── reactive.rs         # Beat-Reactive
│   ├── output/
│   │   ├── mod.rs
│   │   ├── e131.rs             # E1.31/sACN Protokoll
│   │   ├── ddp.rs              # DDP Protokoll
│   │   └── artnet.rs           # Art-Net (optional)
│   └── util/
│       ├── mod.rs
│       └── simd.rs             # SIMD Helpers
├── benches/
│   ├── fft_bench.rs
│   ├── mel_bench.rs
│   └── effect_bench.rs
└── tests/
    └── integration/
```

---

## 3. Work Packages

### WP1: Projekt-Setup & Tooling

**Dauer:** 2-3 Tage  
**Deliverables:** Kompilierbares Skeleton, CI/CD Pipeline

#### WP1.1 Repository & Cargo Setup

- [ ] Cargo.toml mit Workspace-Struktur
- [ ] Feature Flags definieren:
  ```toml
  [features]
  default = ["pipewire", "e131"]
  pipewire = ["dep:pipewire"]
  alsa = ["dep:alsa"]
  e131 = []
  ddp = []
  artnet = []
  simd = []  # Explizite SIMD-Optimierungen
  ```
- [ ] Profile für Release optimieren:
  ```toml
  [profile.release]
  lto = "thin"           # Link-Time Optimization
  codegen-units = 1      # Bessere Optimierung
  panic = "abort"        # Kleinere Binary
  strip = true           # Debug-Symbole entfernen
  opt-level = 3
  
  [profile.release-small]
  inherits = "release"
  opt-level = "s"        # Size statt Speed
  lto = "fat"
  ```

#### WP1.2 Cross-Compilation Setup

- [ ] Docker/Podman Build-Container für ARM
- [ ] `.cargo/config.toml` für Cross-Targets:
  ```toml
  [target.aarch64-unknown-linux-gnu]
  linker = "aarch64-linux-gnu-gcc"
  
  [target.armv7-unknown-linux-gnueabihf]
  linker = "arm-linux-gnueabihf-gcc"
  ```
- [ ] GitHub Actions Workflow für Multi-Arch Builds
- [ ] Release-Binary-Größe Ziel: <2MB stripped

#### WP1.3 Benchmarking-Infrastruktur

- [ ] Criterion-Setup für Mikro-Benchmarks
- [ ] Baseline-Messungen definieren (Zielwerte):

| Operation | Ziel (RPi 4) | Ziel (RPi 3) |
|-----------|-------------|--------------|
| FFT 512pt | <50µs | <100µs |
| Mel Bank | <20µs | <40µs |
| Effect | <30µs | <60µs |
| E1.31 Tx | <10µs | <20µs |
| **Gesamt/Frame** | **<150µs** | **<300µs** |

---

### WP2: Audio Capture (PipeWire)

**Dauer:** 4-5 Tage  
**Deliverables:** Funktionierender Audio-Stream zu Ring Buffer

#### WP2.1 PipeWire Integration

- [ ] `pipewire-rs` Crate evaluieren vs. raw FFI
- [ ] Stream-Setup für Monitor-Capture:
  ```rust
  pub struct AudioCapture {
      core: pipewire::Core,
      stream: pipewire::stream::Stream,
      ring_buffer: Arc<RingBuffer<f32>>,
  }
  
  impl AudioCapture {
      pub fn new(config: &AudioConfig) -> Result<Self> {
          // PipeWire Main Loop in separatem Thread
          // Stream mit spa_audio_info_raw konfigurieren
          // Callback schreibt in Ring Buffer
      }
  }
  ```
- [ ] Konfigurierbare Parameter:
  - Sample Rate: 44100 / 48000 Hz
  - Chunk Size: 256 / 512 / 1024 Samples
  - Format: F32 (intern), Konvertierung von S16/S32
- [ ] Device Enumeration (liste verfügbare Sources)

#### WP2.2 ALSA Fallback (Optional)

- [ ] Feature-Flag `alsa` für Systeme ohne PipeWire
- [ ] Direktes ALSA-Capture für Loopback-Devices
- [ ] Gemeinsames Trait für beide Backends:
  ```rust
  pub trait AudioBackend: Send {
      fn start(&mut self) -> Result<()>;
      fn stop(&mut self) -> Result<()>;
      fn set_device(&mut self, device: &str) -> Result<()>;
      fn sample_rate(&self) -> u32;
  }
  ```

#### WP2.3 Lock-Free Ring Buffer

- [ ] SPSC (Single Producer, Single Consumer) Ring Buffer
- [ ] Cache-Line-aligned für Performance:
  ```rust
  #[repr(C, align(64))]
  pub struct RingBuffer<T> {
      buffer: Box<[T]>,
      capacity: usize,
      write_pos: AtomicUsize,  // Aligned to cache line
      _pad1: [u8; 56],
      read_pos: AtomicUsize,   // Separate cache line
      _pad2: [u8; 56],
  }
  ```
- [ ] Batch-Read für DSP-Thread (lese N samples auf einmal)
- [ ] Overflow-Handling: Drop alte Samples, nicht blockieren

---

### WP3: DSP Pipeline

**Dauer:** 5-7 Tage  
**Deliverables:** FFT, Mel-Bank, Smoothing

#### WP3.1 FFT Implementation

- [ ] Crate-Evaluation:

| Crate | Pro | Contra |
|-------|-----|--------|
| `rustfft` | Pure Rust, portable | Etwas langsamer |
| `realfft` | Optimiert für Real-Input | Auf rustfft basierend |
| `fftw` | Schnellste | Native Dep, Cross-Compile schwieriger |

**Empfehlung:** `realfft` für Balance aus Speed und Portabilität

- [ ] FFT-Wrapper mit Fensterung:
  ```rust
  pub struct FftProcessor {
      fft: Arc<dyn RealToComplex<f32>>,
      window: Box<[f32]>,           // Vorberechnetes Hanning
      input_buffer: Box<[f32]>,
      output_buffer: Box<[Complex<f32>]>,
      magnitude: Box<[f32]>,        // Reusable Output
  }
  
  impl FftProcessor {
      pub fn process(&mut self, samples: &[f32]) -> &[f32] {
          // 1. Windowing (SIMD wenn verfügbar)
          // 2. FFT
          // 3. Magnitude berechnen
          &self.magnitude
      }
  }
  ```
- [ ] Vorberechnung der Hanning-Window-Koeffizienten
- [ ] In-Place Operationen wo möglich

#### WP3.2 Mel-Filterbank

- [ ] Filterbank-Generierung (einmalig bei Init):
  ```rust
  pub struct MelBank {
      filters: Box<[Box<[f32]>]>,   // Sparse wäre besser
      indices: Box<[(usize, usize)]>, // Start/End pro Band
      num_bands: usize,
      output: Box<[f32]>,           // Reusable
  }
  ```
- [ ] Sparse-Repräsentation für Dreiecksfilter (nur Non-Zero speichern)
- [ ] Konfigurierbar: 16/24/32 Bänder, Freq-Range

#### WP3.3 Smoothing & Dynamics

- [ ] Exponential Moving Average:
  ```rust
  pub struct Smoother {
      alpha: f32,
      state: Box<[f32]>,
  }
  
  impl Smoother {
      #[inline]
      pub fn process(&mut self, input: &[f32], output: &mut [f32]) {
          for (i, &x) in input.iter().enumerate() {
              self.state[i] = self.alpha * x + (1.0 - self.alpha) * self.state[i];
              output[i] = self.state[i];
          }
      }
  }
  ```
- [ ] Automatic Gain Control (AGC) für konsistente Levels
- [ ] Attack/Release getrennt konfigurierbar

#### WP3.4 Beat Detection (Optional für v1.0)

- [ ] Onset Detection basierend auf Spectral Flux
- [ ] Adaptive Threshold für verschiedene Genres
- [ ] BPM Estimation via Autocorrelation

---

### WP4: Effect Engine

**Dauer:** 5-6 Tage  
**Deliverables:** Effect Trait, 4-5 Basis-Effekte

#### WP4.1 Effect Trait & Registry

- [ ] Trait Definition:
  ```rust
  pub trait Effect: Send {
      fn name(&self) -> &'static str;
      
      /// Render single frame
      fn render(
          &mut self,
          mel_bands: &[f32],
          beat: Option<f32>,
          output: &mut [Rgb],
      );
      
      /// Dynamische Konfiguration
      fn set_param(&mut self, name: &str, value: f32) -> Result<()>;
      
      /// Reset state (bei Config-Änderung)
      fn reset(&mut self);
  }
  
  #[derive(Clone, Copy)]
  #[repr(C)]
  pub struct Rgb {
      pub r: u8,
      pub g: u8,
      pub b: u8,
  }
  ```
- [ ] Effect Registry für dynamische Auswahl:
  ```rust
  pub struct EffectRegistry {
      effects: HashMap<&'static str, Box<dyn Fn(usize) -> Box<dyn Effect>>>,
  }
  ```

#### WP4.2 Basis-Effekte

| Effekt | Beschreibung | Komplexität |
|--------|--------------|-------------|
| `Energy` | Bass=R, Mid=G, High=B uniform | Niedrig |
| `Spectrum` | Frequenzband pro LED-Segment | Mittel |
| `Scroll` | Neue Werte links, alte scrollen rechts | Mittel |
| `Pulse` | Beat-reaktives Pulsieren | Mittel |
| `Gradient` | Farbverlauf basierend auf Energie | Niedrig |

- [ ] `EnergyEffect`:
  ```rust
  pub struct EnergyEffect {
      num_leds: usize,
      smoothing: Smoother,
      color_map: ColorMap,
  }
  ```

- [ ] `SpectrumEffect`:
  ```rust
  pub struct SpectrumEffect {
      num_leds: usize,
      led_to_band: Box<[usize]>,  // Vorberechnet: welches Band pro LED
      gradient: Gradient,
  }
  ```

- [ ] `ScrollEffect`:
  ```rust
  pub struct ScrollEffect {
      buffer: Box<[Rgb]>,  // History Buffer
      speed: usize,        // Pixels pro Frame
  }
  ```

#### WP4.3 Color Management

- [ ] HSV zu RGB Konvertierung (SIMD-optimierbar)
- [ ] Vordefinierte Gradients (Rainbow, Fire, Ocean, etc.)
- [ ] Gamma-Korrektur für LEDs:
  ```rust
  // LEDs sind nicht linear - Gamma ~2.2-2.8 je nach Strip
  const GAMMA_LUT: [u8; 256] = /* vorberechnet */;
  
  #[inline]
  pub fn gamma_correct(value: u8) -> u8 {
      GAMMA_LUT[value as usize]
  }
  ```

---

### WP5: E1.31/sACN Output

**Dauer:** 4-5 Tage  
**Deliverables:** Vollständiger E1.31 Transmitter

#### WP5.1 E1.31 Protokoll-Implementation

- [ ] Paket-Struktur (125 Bytes Header + DMX Data):
  ```rust
  #[repr(C, packed)]
  pub struct E131Packet {
      // Root Layer (38 bytes)
      preamble_size: u16,           // 0x0010
      postamble_size: u16,          // 0x0000
      acn_id: [u8; 12],             // "ASC-E1.17\0\0\0"
      root_flags_length: u16,       // 0x7000 | length
      root_vector: u32,             // 0x00000004
      cid: [u8; 16],                // Component ID (UUID)
      
      // Framing Layer (77 bytes)
      frame_flags_length: u16,
      frame_vector: u32,            // 0x00000002
      source_name: [u8; 64],
      priority: u8,
      sync_address: u16,
      sequence: u8,
      options: u8,
      universe: u16,
      
      // DMP Layer (10+ bytes)
      dmp_flags_length: u16,
      dmp_vector: u8,               // 0x02
      address_type: u8,             // 0xA1
      first_property_addr: u16,     // 0x0000
      address_increment: u16,       // 0x0001
      property_value_count: u16,
      start_code: u8,               // 0x00
      // DMX data follows (1-512 bytes)
  }
  ```

- [ ] Paket-Builder mit Zero-Copy:
  ```rust
  pub struct E131Sender {
      socket: UdpSocket,
      packet_buffer: Box<[u8; 638]>,  // Max E1.31 packet
      sequence: u8,
      cid: [u8; 16],
  }
  
  impl E131Sender {
      pub fn send_universe(
          &mut self,
          universe: u16,
          data: &[u8],  // Max 512 bytes
      ) -> io::Result<()> {
          // Header nur einmal initialisiert, nur dynamische Felder updaten
          self.update_sequence();
          self.update_universe(universe);
          self.update_data(data);
          self.socket.send(&self.packet_buffer[..self.packet_len()])
      }
  }
  ```

#### WP5.2 Multi-Universe Support

- [ ] Automatisches Splitting bei >170 RGB LEDs:
  ```rust
  pub struct MultiUniverseOutput {
      senders: Vec<E131Sender>,
      base_universe: u16,
      leds_per_universe: usize,  // 170 für RGB
  }
  
  impl MultiUniverseOutput {
      pub fn send(&mut self, led_data: &[Rgb]) -> io::Result<()> {
          for (i, chunk) in led_data.chunks(self.leds_per_universe).enumerate() {
              let universe = self.base_universe + i as u16;
              self.senders[i].send_universe(universe, chunk.as_bytes())?;
          }
          Ok(())
      }
  }
  ```
- [ ] Universe-Discovery (optional, für Controller-Auto-Config)

#### WP5.3 Multicast vs. Unicast

- [ ] Multicast-Adressen nach E1.31 Standard:
  ```rust
  // E1.31 Multicast: 239.255.{high}.{low}
  fn universe_to_multicast(universe: u16) -> Ipv4Addr {
      Ipv4Addr::new(239, 255, (universe >> 8) as u8, universe as u8)
  }
  ```
- [ ] Unicast-Option für direkte Verbindung
- [ ] Konfigurierbare TTL für Multicast

#### WP5.4 DDP Support (Alternative zu E1.31)

- [ ] DDP Protokoll (simpler als E1.31):
  ```rust
  pub struct DdpSender {
      socket: UdpSocket,
      target: SocketAddr,
      sequence: u8,
  }
  
  impl DdpSender {
      pub fn send(&mut self, data: &[Rgb]) -> io::Result<()> {
          let header = [
              0x41,                    // Flags
              self.sequence,
              0x01,                    // RGB
              0x00,                    // Device ID
              0, 0, 0, 0,              // Offset
              (data.len() >> 8) as u8,
              data.len() as u8,
          ];
          // Send header + data
      }
  }
  ```

#### WP5.5 Rate Limiting & Timing

- [ ] Konfigurierbares FPS-Limit (30/60/120)
- [ ] Präzises Timing mit `std::thread::sleep` oder `spin_sleep`
- [ ] Frame-Skipping bei Überlast statt Stau

---

### WP6: Konfiguration & CLI

**Dauer:** 2-3 Tage  
**Deliverables:** TOML Config, CLI Interface

#### WP6.1 Konfigurationsschema

- [ ] TOML-basierte Konfiguration:
  ```toml
  # rustled.toml
  
  [audio]
  backend = "pipewire"        # pipewire | alsa
  device = "auto"             # "auto" oder Device-Name
  sample_rate = 48000
  chunk_size = 512
  
  [dsp]
  fft_size = 512              # 256 | 512 | 1024
  mel_bands = 24
  freq_min = 20
  freq_max = 18000
  smoothing = 0.7             # 0.0 - 1.0
  
  [effect]
  name = "spectrum"
  
  [effect.params]
  gradient = "rainbow"
  mirror = false
  
  [output]
  protocol = "e131"           # e131 | ddp | artnet
  target = "239.255.0.1"      # Multicast oder IP
  universe = 1
  fps = 60
  
  [output.leds]
  count = 300
  rgb_order = "GRB"           # RGB | GRB | BGR | ...
  
  [[output.segments]]         # Optional: mehrere Outputs
  target = "192.168.1.100"
  universe = 1
  start_led = 0
  end_led = 150
  ```

- [ ] Serde-Strukturen mit Defaults:
  ```rust
  #[derive(Deserialize)]
  #[serde(default)]
  pub struct Config {
      pub audio: AudioConfig,
      pub dsp: DspConfig,
      pub effect: EffectConfig,
      pub output: OutputConfig,
  }
  
  impl Default for DspConfig {
      fn default() -> Self {
          Self {
              fft_size: 512,
              mel_bands: 24,
              // ...
          }
      }
  }
  ```

#### WP6.2 CLI Interface

- [ ] `clap` für Argument Parsing:
  ```
  rustled 0.1.0
  Audio-reactive LED controller
  
  USAGE:
      rustled [OPTIONS] [SUBCOMMAND]
  
  OPTIONS:
      -c, --config <FILE>     Config file [default: rustled.toml]
      -d, --device <NAME>     Audio device (overrides config)
      -e, --effect <NAME>     Effect name (overrides config)
      -v, --verbose           Increase logging
      -q, --quiet             Suppress output
  
  SUBCOMMANDS:
      run         Start the visualizer (default)
      devices     List audio devices
      effects     List available effects
      test        Send test pattern to LEDs
  ```

#### WP6.3 Hot-Reload (Optional)

- [ ] Config-File-Watcher mit `notify`
- [ ] Graceful Reload ohne Audio-Unterbrechung

---

### WP7: Integration & Testing

**Dauer:** 3-4 Tage  
**Deliverables:** Lauffähiges Gesamtsystem

#### WP7.1 Integration

- [ ] Main Loop Assembly:
  ```rust
  fn main() -> Result<()> {
      let config = Config::load()?;
      
      // Channels zwischen Threads
      let (audio_tx, audio_rx) = spsc::channel(4096);
      let (led_tx, led_rx) = spsc::channel(64);
      
      // Audio Thread (RT Priority)
      let audio_handle = thread::Builder::new()
          .name("audio".into())
          .spawn(move || audio_thread(config.audio, audio_tx))?;
      
      // DSP + Effect Thread
      let dsp_handle = thread::spawn(move || {
          dsp_thread(config.dsp, config.effect, audio_rx, led_tx)
      });
      
      // Output Thread
      let output_handle = thread::spawn(move || {
          output_thread(config.output, led_rx)
      });
      
      // Signal Handler
      wait_for_shutdown()?;
      
      Ok(())
  }
  ```

#### WP7.2 Testing

- [ ] Unit Tests für jedes Modul
- [ ] Integration Tests:
  - Audio Capture → DSP → Dummy Output
  - Config Loading mit verschiedenen Szenarien
- [ ] E1.31 Packet Capture & Validation mit Wireshark
- [ ] Performance Tests auf Ziel-Hardware

#### WP7.3 Profiling auf Target

- [ ] `perf` Profiling auf RPi
- [ ] Memory-Analyse mit `heaptrack`
- [ ] Identifikation von Hotspots

---

### WP8: Optimierung

**Dauer:** 3-5 Tage  
**Deliverables:** Optimiertes Release-Build

#### WP8.1 SIMD-Optimierungen

- [ ] Portable SIMD für kritische Loops:
  ```rust
  #[cfg(target_arch = "aarch64")]
  use std::arch::aarch64::*;
  
  #[cfg(target_arch = "x86_64")]
  use std::arch::x86_64::*;
  
  // Beispiel: Windowing mit NEON/SSE
  pub fn apply_window_simd(samples: &mut [f32], window: &[f32]) {
      #[cfg(target_feature = "neon")]
      unsafe {
          // NEON Implementation
      }
      
      #[cfg(target_feature = "sse")]
      unsafe {
          // SSE Implementation
      }
      
      #[cfg(not(any(target_feature = "neon", target_feature = "sse")))]
      {
          // Scalar Fallback
          for (s, w) in samples.iter_mut().zip(window) {
              *s *= w;
          }
      }
  }
  ```

#### WP8.2 Memory-Optimierung

- [ ] Alle Allocations bei Init, keine zur Runtime
- [ ] Reusable Buffers in allen Modulen
- [ ] Stack-Allokation für kleine Arrays
- [ ] Ziel: 0 Allocations pro Frame

#### WP8.3 32-bit ARMv7 Spezifika

- [ ] `soft-float` vs `hard-float` ABI testen
- [ ] VFPv3/VFPv4 Feature Detection
- [ ] Fallback für ältere RPi ohne NEON

---

### WP9: Dokumentation & Release

**Dauer:** 2-3 Tage  
**Deliverables:** README, Release Binaries

#### WP9.1 Dokumentation

- [ ] README.md mit Quick Start
- [ ] INSTALL.md für verschiedene Plattformen
- [ ] CONFIG.md mit allen Optionen
- [ ] EFFECTS.md mit Effekt-Beschreibungen
- [ ] Inline Rust-Dokumentation (`///`)

#### WP9.2 Release Pipeline

- [ ] GitHub Releases mit vorgebauten Binaries:
  - `rustled-linux-x86_64`
  - `rustled-linux-aarch64`
  - `rustled-linux-armv7hf`
- [ ] Checksums (SHA256)
- [ ] Changelog generieren

#### WP9.3 Packaging (Optional)

- [ ] `.deb` Package für Debian/Ubuntu/RPi OS
- [ ] systemd Service Unit:
  ```ini
  [Unit]
  Description=RustLED Audio Visualizer
  After=pipewire.service
  
  [Service]
  ExecStart=/usr/bin/rustled -c /etc/rustled/config.toml
  Restart=on-failure
  User=rustled
  Group=audio
  
  [Install]
  WantedBy=multi-user.target
  ```

---

## 4. Abhängigkeiten & Crates

### Kern-Dependencies

| Crate | Version | Zweck | Größe |
|-------|---------|-------|-------|
| `pipewire` | 0.8 | Audio Capture | ~50KB |
| `realfft` | 3.x | FFT | ~30KB |
| `serde` | 1.x | Config Parsing | ~60KB |
| `toml` | 0.8 | TOML Parser | ~40KB |
| `clap` | 4.x | CLI | ~100KB |
| `log` + `env_logger` | 0.4/0.11 | Logging | ~30KB |
| `socket2` | 0.5 | UDP Sockets | ~20KB |

### Optionale Dependencies

| Crate | Feature | Zweck |
|-------|---------|-------|
| `alsa` | `alsa` | ALSA Fallback |
| `notify` | `hot-reload` | Config Reload |
| `criterion` | dev | Benchmarking |

### Vermeidete Dependencies

- `tokio` / `async-std` – Overhead für simples Threading unnötig
- `crossbeam` – `std::sync` reicht für SPSC
- `num-complex` – Nur `rustfft::num_complex` re-export nutzen

---

## 5. Ressourcenoptimierung

### 5.1 CPU-Budget pro Frame (60 FPS = 16.6ms)

```
┌────────────────────────────────────────────────────────┐
│                    Frame Budget                         │
├─────────────────┬──────────────┬───────────────────────┤
│ Operation       │ Target (µs)  │ Max Allowed (µs)      │
├─────────────────┼──────────────┼───────────────────────┤
│ Audio Read      │     10       │       50              │
│ FFT 512pt       │     40       │      100              │
│ Mel Bank        │     15       │       40              │
│ Effect Render   │     25       │       80              │
│ E1.31 Packet    │      5       │       20              │
│ UDP Send        │      5       │       20              │
├─────────────────┼──────────────┼───────────────────────┤
│ TOTAL           │    100       │      310              │
│ Headroom        │  16500       │    16290              │
└─────────────────┴──────────────┴───────────────────────┘
```

### 5.2 Memory Budget

| Komponente | Größe | Anmerkung |
|------------|-------|-----------|
| Audio Ring Buffer | 32 KB | 8192 samples × 4 bytes |
| FFT Buffers | 8 KB | Input + Output + Scratch |
| Mel Filterbank | 4 KB | 24 Bands × ~40 Koeffizienten |
| LED Buffer | 1 KB | 300 LEDs × 3 bytes × 2 |
| E1.31 Packet | 2 KB | 2× für Double-Buffering |
| **Static Total** | **~50 KB** | |
| Binary Size | <2 MB | Stripped Release |
| Runtime RSS | <10 MB | Inkl. Shared Libs |

### 5.3 Latenz-Budget

```
Audio Event → LED Output: < 30ms (imperceptible)

┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐
│ PipeWire │──▶│   DSP    │──▶│  Effect  │──▶│  E1.31   │
│  ~11ms   │   │   ~1ms   │   │   ~1ms   │   │   ~1ms   │
└──────────┘   └──────────┘   └──────────┘   └──────────┘
                                                   │
                                                   ▼
                                            ┌──────────┐
                                            │ Network  │
                                            │  ~1-5ms  │
                                            └──────────┘
                                                   │
                                                   ▼
                                            ┌──────────┐
                                            │   WLED   │
                                            │  ~5-10ms │
                                            └──────────┘
```

---

## 6. Build & Cross-Compilation

### 6.1 Native Build

```bash
# Debug
cargo build

# Release
cargo build --release

# Minimal Size
cargo build --profile release-small
```

### 6.2 Cross-Compilation für Raspberry Pi

```bash
# Setup (einmalig)
rustup target add aarch64-unknown-linux-gnu
rustup target add armv7-unknown-linux-gnueabihf

# Install Cross-Compiler
# Debian/Ubuntu:
sudo apt install gcc-aarch64-linux-gnu gcc-arm-linux-gnueabihf

# Build für RPi 4/5 (64-bit)
cargo build --release --target aarch64-unknown-linux-gnu

# Build für RPi 2/3 (32-bit)
cargo build --release --target armv7-unknown-linux-gnueabihf
```

### 6.3 Docker Cross-Build (empfohlen)

```dockerfile
# Dockerfile.cross
FROM rust:1.75-bookworm

RUN dpkg --add-architecture arm64 && \
    dpkg --add-architecture armhf && \
    apt-get update && \
    apt-get install -y \
        gcc-aarch64-linux-gnu \
        gcc-arm-linux-gnueabihf \
        libpipewire-0.3-dev:arm64 \
        libpipewire-0.3-dev:armhf

RUN rustup target add aarch64-unknown-linux-gnu armv7-unknown-linux-gnueabihf

WORKDIR /build
```

```bash
# Build-Script
docker build -t rustled-cross -f Dockerfile.cross .
docker run -v $(pwd):/build rustled-cross \
    cargo build --release --target aarch64-unknown-linux-gnu
```

### 6.4 GitHub Actions Workflow

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags: ['v*']

jobs:
  build:
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
          - target: armv7-unknown-linux-gnueabihf
            os: ubuntu-latest

    runs-on: ${{ matrix.os }}
    
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - name: Build
        run: cargo build --release --target ${{ matrix.target }}
      - name: Upload
        uses: actions/upload-artifact@v4
        with:
          name: rustled-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/rustled
```

---

## 7. Zeitschätzung

### Gesamtübersicht

| Work Package | Geschätzte Dauer | Abhängigkeiten |
|--------------|------------------|----------------|
| WP1: Setup | 2-3 Tage | – |
| WP2: Audio Capture | 4-5 Tage | WP1 |
| WP3: DSP Pipeline | 5-7 Tage | WP1 |
| WP4: Effect Engine | 5-6 Tage | WP3 |
| WP5: E1.31 Output | 4-5 Tage | WP1 |
| WP6: Config & CLI | 2-3 Tage | WP1 |
| WP7: Integration | 3-4 Tage | WP2-WP6 |
| WP8: Optimierung | 3-5 Tage | WP7 |
| WP9: Dokumentation | 2-3 Tage | WP7 |

### Kritischer Pfad

```
WP1 ──▶ WP2 ──┐
              ├──▶ WP7 ──▶ WP8 ──▶ WP9
WP1 ──▶ WP3 ──▶ WP4 ──┤
                      │
WP1 ──▶ WP5 ──────────┤
                      │
WP1 ──▶ WP6 ──────────┘
```

### Timeline (bei ~4h/Tag Arbeitszeit)

| Woche | Work Packages | Milestone |
|-------|---------------|-----------|
| 1 | WP1, WP2 Start | Projekt Setup, Audio Stream |
| 2 | WP2 Ende, WP3 Start | Audio funktioniert |
| 3 | WP3, WP4 Start | FFT + Mel Bank fertig |
| 4 | WP4, WP5 | Effekte + E1.31 |
| 5 | WP6, WP7 | Config, Integration |
| 6 | WP8, WP9 | Optimierung, Release |

**Geschätzte Gesamtdauer: 5-7 Wochen**

---

## 8. Risiken & Mitigationen

### R1: PipeWire API Instabilität

**Risiko:** `pipewire-rs` Crate hat Breaking Changes  
**Wahrscheinlichkeit:** Mittel  
**Impact:** Hoch  
**Mitigation:**
- Version pinnen in `Cargo.toml`
- ALSA-Fallback als Feature
- PipeWire über C-FFI direkt ansprechen als letzte Option

### R2: Cross-Compilation Komplexität

**Risiko:** Native Dependencies (PipeWire) erschweren Cross-Build  
**Wahrscheinlichkeit:** Hoch  
**Impact:** Mittel  
**Mitigation:**
- Docker-basierter Build mit Multi-Arch Images
- Statisches Linking wo möglich
- CI mit echten ARM-Runnern (falls verfügbar)

### R3: Performance auf RPi 2/Zero

**Risiko:** 32-bit ARMv6/v7 zu langsam für 60 FPS  
**Wahrscheinlichkeit:** Mittel  
**Impact:** Niedrig (nicht primäre Zielplattform)  
**Mitigation:**
- FPS reduzierbar (30 FPS Modus)
- FFT-Größe reduzieren (256pt)
- Effekt-Komplexität skalierbar

### R4: E1.31 Interoperabilität

**Risiko:** Verschiedene Controller interpretieren E1.31 unterschiedlich  
**Wahrscheinlichkeit:** Niedrig  
**Impact:** Mittel  
**Mitigation:**
- Strikte Protokoll-Compliance nach ANSI E1.31-2018
- Test mit WLED, xLights, OLA
- DDP als simpler Fallback

### R5: Audio-Latenz

**Risiko:** PipeWire-Latenz zu hoch für reaktive Visualisierung  
**Wahrscheinlichkeit:** Niedrig  
**Impact:** Hoch  
**Mitigation:**
- Buffer-Größen minimieren
- RT-Thread-Priority für Audio
- Direct ALSA als Ultra-Low-Latency Option

---

## Anhang A: E1.31 Protokoll-Referenz

### A.1 Packet Layout

```
Offset  Size  Field                    Value
------  ----  -----                    -----
0       2     Preamble Size            0x0010
2       2     Postamble Size           0x0000
4       12    ACN Packet Identifier    "ASC-E1.17\0\0\0"
16      2     Flags + Length           0x7000 | (length - 16)
18      4     Root Vector              0x00000004
22      16    CID                      UUID
38      2     Framing Flags + Length   0x7000 | (length - 38)
40      4     Framing Vector           0x00000002
44      64    Source Name              UTF-8, null-padded
108     1     Priority                 0-200 (100 default)
109     2     Sync Address             0x0000
111     1     Sequence Number          0-255, incrementing
112     1     Options                  0x00
113     2     Universe                 1-63999
115     2     DMP Flags + Length       0x7000 | (length - 115)
117     1     DMP Vector               0x02
118     1     Address/Data Type        0xA1
119     2     First Property Address   0x0000
121     2     Address Increment        0x0001
123     2     Property Value Count     1-513
125     1     Start Code               0x00 (DMX512)
126     N     DMX Data                 1-512 bytes
```

### A.2 Multicast-Adressen

```
Universe 1:     239.255.0.1
Universe 256:   239.255.1.0
Universe 63999: 239.255.249.255

Formel: 239.255.{(universe >> 8) & 0xFF}.{universe & 0xFF}
```

---

## Anhang B: Referenz-Benchmarks

### B.1 FFT Performance (zu erwarten)

| Platform | FFT 512pt | FFT 1024pt |
|----------|-----------|------------|
| RPi 5 | ~15µs | ~35µs |
| RPi 4 | ~25µs | ~55µs |
| RPi 3 (64-bit) | ~45µs | ~100µs |
| RPi 3 (32-bit) | ~60µs | ~130µs |
| x86_64 (i7) | ~5µs | ~12µs |

### B.2 Vergleich mit LedFX (Python)

| Metrik | LedFX (Python) | RustLED (Ziel) |
|--------|----------------|----------------|
| CPU Usage (RPi 4) | 25-40% | <5% |
| RAM Usage | ~150MB | <10MB |
| Startup Time | 3-5s | <0.5s |
| Binary Size | ~50MB (mit deps) | <2MB |
| Frame Latency | ~30ms | <15ms |

---

## Anhang C: Glossar

| Begriff | Beschreibung |
|---------|--------------|
| **E1.31/sACN** | Streaming ACN, Protokoll für DMX über IP |
| **DDP** | Distributed Display Protocol, simples LED-Protokoll |
| **FFT** | Fast Fourier Transform, Zeitbereich → Frequenzbereich |
| **Mel-Skala** | Logarithmische Frequenzskala (menschliches Hören) |
| **SPSC** | Single Producer Single Consumer (Queue-Typ) |
| **Universe** | E1.31 Kanal für 512 DMX-Werte (170 RGB LEDs) |
| **WLED** | Open-Source LED-Controller-Firmware |

---

## Anhang D: Weiterführende Ressourcen

- [E1.31 Spezifikation (ANSI E1.31-2018)](https://tsp.esta.org/tsp/documents/docs/ANSI_E1-31-2018.pdf)
- [WLED Dokumentation](https://kno.wled.ge/)
- [PipeWire Dokumentation](https://docs.pipewire.org/)
- [Rust FFT Benchmarks](https://github.com/ejmahler/rust_fft_bench)
- [Mel-Filterbank Theorie](https://haythamfayek.com/2016/04/21/speech-processing-for-machine-learning.html)
