//! FIFO audio backend for MPD and other players that output raw PCM to a named pipe

use super::{AudioBackend, AudioError, RingBuffer};
use std::fs::File;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// FIFO audio backend
///
/// Reads raw PCM samples (S16LE stereo) from a named pipe, e.g. `/tmp/mpd.fifo`.
pub struct FifoCapture {
    sample_rate: u32,
    channels: u32,
    fifo_path: String,
    ring_buffer: Arc<RingBuffer<f32>>,
    running: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

impl FifoCapture {
    /// Create a new FIFO capture instance
    ///
    /// # Arguments
    /// * `sample_rate` - Sample rate of the PCM data (must match MPD config)
    /// * `ring_buffer` - Shared ring buffer for audio samples
    /// * `fifo_path` - Path to the named pipe (e.g. "/tmp/mpd.fifo")
    pub fn new(
        sample_rate: u32,
        ring_buffer: Arc<RingBuffer<f32>>,
        fifo_path: &str,
    ) -> Result<Self, AudioError> {
        Ok(Self {
            sample_rate,
            channels: 2,
            fifo_path: fifo_path.to_string(),
            ring_buffer,
            running: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
        })
    }
}

impl AudioBackend for FifoCapture {
    fn start(&mut self) -> Result<(), AudioError> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.running.store(true, Ordering::SeqCst);

        let channels = self.channels;
        let ring_buffer = self.ring_buffer.clone();
        let running = self.running.clone();
        let fifo_path = self.fifo_path.clone();

        let handle = thread::Builder::new()
            .name("fifo-audio".to_string())
            .spawn(move || {
                if let Err(e) = run_capture_loop(channels, ring_buffer, running, &fifo_path) {
                    log::error!("FIFO capture error: {}", e);
                }
            })
            .map_err(|e| AudioError::InitError(format!("Failed to spawn thread: {}", e)))?;

        self.thread_handle = Some(handle);

        log::info!(
            "FIFO capture started: {}Hz, {} channels, path: {}",
            self.sample_rate,
            self.channels,
            self.fifo_path
        );

        Ok(())
    }

    fn stop(&mut self) -> Result<(), AudioError> {
        if !self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.running.store(false, Ordering::SeqCst);

        if let Some(handle) = self.thread_handle.take() {
            handle
                .join()
                .map_err(|_| AudioError::StreamError("Thread join failed".to_string()))?;
        }

        log::info!("FIFO capture stopped");
        Ok(())
    }

    fn set_device(&mut self, device: &str) -> Result<(), AudioError> {
        self.fifo_path = device.to_string();
        Ok(())
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

impl Drop for FifoCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Read raw S16LE PCM from a named pipe and push mono f32 samples to the ring buffer
fn run_capture_loop(
    channels: u32,
    ring_buffer: Arc<RingBuffer<f32>>,
    running: Arc<AtomicBool>,
    fifo_path: &str,
) -> Result<(), AudioError> {
    log::info!("Opening FIFO: {} (will block until writer connects)", fifo_path);

    // Read size: 512 frames * 2 channels * 2 bytes = 2048 bytes
    let frame_bytes = channels as usize * std::mem::size_of::<i16>();
    let buf_frames = 512;
    let mut buf = vec![0u8; buf_frames * frame_bytes];

    while running.load(Ordering::Relaxed) {
        // Open the FIFO (blocks until a writer opens the other end).
        // We re-open on EOF so we survive MPD restarts.
        let mut file = match File::open(fifo_path) {
            Ok(f) => f,
            Err(e) => {
                log::error!("Failed to open FIFO '{}': {}", fifo_path, e);
                // Wait a bit before retrying to avoid busy-loop
                thread::sleep(std::time::Duration::from_secs(1));
                continue;
            }
        };

        log::info!("FIFO connected, reading PCM data");

        while running.load(Ordering::Relaxed) {
            match file.read(&mut buf) {
                Ok(0) => {
                    // EOF — writer closed (e.g. MPD stopped). Re-open.
                    log::debug!("FIFO EOF, waiting for writer to reconnect");
                    break;
                }
                Ok(n) => {
                    // Process complete frames only
                    let complete_bytes = n - (n % frame_bytes);
                    let samples = &buf[..complete_bytes];

                    for frame in samples.chunks_exact(frame_bytes) {
                        let mut mono: f32 = 0.0;
                        for ch in 0..channels as usize {
                            let offset = ch * 2;
                            let sample = i16::from_le_bytes([frame[offset], frame[offset + 1]]);
                            mono += sample as f32 / 32768.0;
                        }
                        mono /= channels as f32;
                        ring_buffer.push(mono);
                    }
                }
                Err(e) => {
                    log::warn!("FIFO read error: {}, retrying", e);
                    break;
                }
            }
        }
    }

    Ok(())
}
