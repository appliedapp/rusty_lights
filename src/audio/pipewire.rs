//! PipeWire audio capture backend

use super::{AudioBackend, AudioError, RingBuffer};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use pipewire as pw;
use pw::spa::param::audio::AudioInfoRaw;
use pw::spa::param::format::{MediaSubtype, MediaType};
use pw::spa::param::format_utils;
use pw::spa::utils::Direction;
use pw::stream::StreamFlags;

/// PipeWire audio capture backend
pub struct PipeWireCapture {
    sample_rate: u32,
    channels: u32,
    ring_buffer: Arc<RingBuffer<f32>>,
    device: String,
    running: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

impl PipeWireCapture {
    /// Create a new PipeWire capture instance
    pub fn new(sample_rate: u32, ring_buffer: Arc<RingBuffer<f32>>) -> Result<Self, AudioError> {
        pw::init();

        Ok(Self {
            sample_rate,
            channels: 2,
            ring_buffer,
            device: "auto".to_string(),
            running: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
        })
    }

    /// List available audio devices
    pub fn list_devices() -> Result<Vec<AudioDevice>, AudioError> {
        pw::init();

        let mainloop = pw::main_loop::MainLoopRc::new(None)
            .map_err(|e| AudioError::InitError(format!("Failed to create main loop: {}", e)))?;

        let context = pw::context::ContextRc::new(&mainloop, None)
            .map_err(|e| AudioError::InitError(format!("Failed to create context: {}", e)))?;

        let core = context
            .connect_rc(None)
            .map_err(|e| AudioError::InitError(format!("Failed to connect: {}", e)))?;

        let registry = core
            .get_registry()
            .map_err(|e| AudioError::InitError(format!("Failed to get registry: {}", e)))?;

        let devices = Arc::new(std::sync::Mutex::new(Vec::new()));
        let devices_clone = devices.clone();

        let _listener = registry
            .add_listener_local()
            .global(move |global| {
                if global.type_ == pw::types::ObjectType::Node {
                    if let Some(props) = global.props {
                        let media_class: Option<&str> = props.get("media.class");
                        let name: Option<&str> = props.get("node.name");

                        if let (Some(media_class), Some(name)) = (media_class, name) {
                            let description: &str = props
                                .get("node.description")
                                .unwrap_or(name);

                            if media_class.contains("Audio")
                                && (media_class.contains("Source")
                                    || media_class.contains("Sink"))
                            {
                                let mut devs = devices_clone.lock().unwrap();
                                devs.push(AudioDevice {
                                    id: global.id,
                                    name: name.to_string(),
                                    description: description.to_string(),
                                    is_monitor: media_class.contains("Monitor"),
                                });
                            }
                        }
                    }
                }
            })
            .register();

        // Iterate briefly to enumerate devices (100ms timeout, 10 iterations)
        for _ in 0..10 {
            mainloop.loop_().iterate(std::time::Duration::from_millis(10));
        }

        let result = devices.lock().unwrap().clone();
        Ok(result)
    }
}

impl AudioBackend for PipeWireCapture {
    fn start(&mut self) -> Result<(), AudioError> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.running.store(true, Ordering::SeqCst);

        let sample_rate = self.sample_rate;
        let channels = self.channels;
        let ring_buffer = self.ring_buffer.clone();
        let running = self.running.clone();
        let device = self.device.clone();

        let handle = thread::Builder::new()
            .name("pipewire-audio".to_string())
            .spawn(move || {
                if let Err(e) = run_capture_loop(sample_rate, channels, ring_buffer, running, &device) {
                    log::error!("PipeWire capture error: {}", e);
                }
            })
            .map_err(|e| AudioError::InitError(format!("Failed to spawn thread: {}", e)))?;

        self.thread_handle = Some(handle);

        log::info!("PipeWire capture started: {}Hz, {} channels", self.sample_rate, self.channels);
        Ok(())
    }

    fn stop(&mut self) -> Result<(), AudioError> {
        if !self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.running.store(false, Ordering::SeqCst);

        if let Some(handle) = self.thread_handle.take() {
            handle.join().map_err(|_| AudioError::StreamError("Thread join failed".to_string()))?;
        }

        log::info!("PipeWire capture stopped");
        Ok(())
    }

    fn set_device(&mut self, device: &str) -> Result<(), AudioError> {
        self.device = device.to_string();
        Ok(())
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

impl Drop for PipeWireCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[derive(Debug, Clone)]
pub struct AudioDevice {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub is_monitor: bool,
}

struct CaptureData {
    format: AudioInfoRaw,
    ring_buffer: Arc<RingBuffer<f32>>,
    running: Arc<AtomicBool>,
}

fn run_capture_loop(
    _sample_rate: u32,
    _channels: u32,
    ring_buffer: Arc<RingBuffer<f32>>,
    running: Arc<AtomicBool>,
    device: &str,
) -> Result<(), AudioError> {
    let mainloop = pw::main_loop::MainLoopRc::new(None)
        .map_err(|e| AudioError::InitError(format!("Failed to create main loop: {}", e)))?;

    let context = pw::context::ContextRc::new(&mainloop, None)
        .map_err(|e| AudioError::InitError(format!("Failed to create context: {}", e)))?;

    let core = context
        .connect_rc(None)
        .map_err(|e| AudioError::InitError(format!("Failed to connect: {}", e)))?;

    let mut props = pw::properties::properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Capture",
        *pw::keys::MEDIA_ROLE => "Music",
    };

    // Capture from sink monitor (system audio output)
    props.insert(*pw::keys::STREAM_CAPTURE_SINK, "true");

    if device != "auto" {
        // Use node.target for specifying target device
        props.insert("node.target", device);
    }

    let stream = pw::stream::StreamBox::new(&core, "rusty_lights", props)
        .map_err(|e| AudioError::InitError(format!("Failed to create stream: {}", e)))?;

    let data = CaptureData {
        format: AudioInfoRaw::default(),
        ring_buffer,
        running: running.clone(),
    };

    let loop_weak = mainloop.downgrade();

    let _listener = stream
        .add_local_listener_with_user_data(data)
        .param_changed(|_, user_data, id, param| {
            let Some(param) = param else { return };
            if id != pw::spa::param::ParamType::Format.as_raw() {
                return;
            }

            let Ok((media_type, media_subtype)) = format_utils::parse_format(param) else {
                return;
            };

            if media_type != MediaType::Audio || media_subtype != MediaSubtype::Raw {
                return;
            }

            if user_data.format.parse(param).is_ok() {
                log::info!(
                    "PipeWire format: rate={} channels={}",
                    user_data.format.rate(),
                    user_data.format.channels()
                );
            }
        })
        .process(move |stream, user_data| {
            if !user_data.running.load(Ordering::Relaxed) {
                if let Some(ml) = loop_weak.upgrade() {
                    ml.quit();
                }
                return;
            }

            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };

            let datas = buffer.datas_mut();
            if datas.is_empty() {
                return;
            }

            let data = &mut datas[0];
            if let Some(samples) = data.data() {
                let n_channels = user_data.format.channels().max(1);
                let sample_size = std::mem::size_of::<f32>();

                // Process F32LE samples
                for chunk in samples.chunks_exact(sample_size * n_channels as usize) {
                    let mut mono: f32 = 0.0;
                    for c in 0..n_channels as usize {
                        let start = c * sample_size;
                        let end = start + sample_size;
                        if end <= chunk.len() {
                            let bytes: [u8; 4] = chunk[start..end].try_into().unwrap_or([0; 4]);
                            mono += f32::from_le_bytes(bytes);
                        }
                    }
                    mono /= n_channels as f32;
                    user_data.ring_buffer.push(mono);
                }
            }
        })
        .register();

    // Connect with no specific format (let PipeWire negotiate)
    let mut params = [];

    stream
        .connect(
            Direction::Input,
            None,
            StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS | StreamFlags::RT_PROCESS,
            &mut params,
        )
        .map_err(|e| AudioError::InitError(format!("Failed to connect stream: {}", e)))?;

    log::info!("PipeWire stream connected");

    // Run the main loop (blocks until quit is called)
    mainloop.run();

    Ok(())
}
