//! HTTP server with WebSocket upgrade and broadcast dispatcher

use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use crate::effects::Rgb;
use crate::engine::ConfigUpdateMsg;

use super::protocol::encode_frame;
use super::websocket::run_client;

const UI_HTML: &str = include_str!("ui.html");

/// Frame data sent from the processing loop
pub type FrameData = (Vec<Rgb>, Vec<f32>, Option<f32>);

/// Start the HTTP/WebSocket server.
pub fn start(
    port: u16,
    frame_rx: mpsc::Receiver<FrameData>,
    config_tx: mpsc::Sender<ConfigUpdateMsg>,
    running: Arc<AtomicBool>,
    initial_effect: String,
    initial_brightness: f32,
    initial_smoothing: f32,
    initial_beat_sensitivity: f32,
) -> std::io::Result<thread::JoinHandle<()>> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
    listener.set_nonblocking(true)?;

    log::info!("Web UI available at http://localhost:{}", port);

    let handle = thread::Builder::new()
        .name("web-server".to_string())
        .spawn(move || {
            run_server(
                listener,
                frame_rx,
                config_tx,
                running,
                initial_effect,
                initial_brightness,
                initial_smoothing,
                initial_beat_sensitivity,
            );
        })
        .expect("Failed to spawn web server thread");

    Ok(handle)
}

fn run_server(
    listener: TcpListener,
    frame_rx: mpsc::Receiver<FrameData>,
    config_tx: mpsc::Sender<ConfigUpdateMsg>,
    running: Arc<AtomicBool>,
    initial_effect: String,
    initial_brightness: f32,
    initial_smoothing: f32,
    initial_beat_sensitivity: f32,
) {
    let (client_reg_tx, client_reg_rx) = mpsc::channel::<mpsc::SyncSender<Vec<u8>>>();

    let config_state = Arc::new(std::sync::Mutex::new(build_config_json(
        &initial_effect,
        initial_brightness,
        initial_smoothing,
        initial_beat_sensitivity,
    )));

    // Spawn broadcast dispatcher thread
    let running_bc = running.clone();
    thread::Builder::new()
        .name("web-broadcast".to_string())
        .spawn(move || {
            run_broadcast(frame_rx, client_reg_rx, running_bc);
        })
        .expect("Failed to spawn broadcast thread");

    while running.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _addr)) => {
                let config_tx = config_tx.clone();
                let client_reg_tx = client_reg_tx.clone();
                let config_state = config_state.clone();

                thread::Builder::new()
                    .name("web-conn".to_string())
                    .spawn(move || {
                        handle_connection(stream, config_tx, client_reg_tx, config_state);
                    })
                    .ok();
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => {
                log::warn!("TCP accept error: {}", e);
            }
        }
    }
}

fn handle_connection(
    stream: TcpStream,
    config_tx: mpsc::Sender<ConfigUpdateMsg>,
    client_reg_tx: mpsc::Sender<mpsc::SyncSender<Vec<u8>>>,
    config_state: Arc<std::sync::Mutex<String>>,
) {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));

    // Peek at the request line to determine if this is a WebSocket request
    // without consuming the bytes (so tungstenite can re-read them)
    let mut peek_buf = [0u8; 512];
    let n = match stream.peek(&mut peek_buf) {
        Ok(n) => n,
        Err(_) => return,
    };

    let request_preview = String::from_utf8_lossy(&peek_buf[..n]);

    if request_preview.contains("/ws") && request_preview.to_lowercase().contains("upgrade") {
        // WebSocket upgrade — let tungstenite handle the full handshake
        let _ = stream.set_read_timeout(None);

        match tungstenite::accept(&stream) {
            Ok(ws) => {
                log::info!("WebSocket client connected");

                let (frame_tx, frame_rx) = mpsc::sync_channel::<Vec<u8>>(2);
                if client_reg_tx.send(frame_tx).is_err() {
                    return;
                }

                let initial_state = config_state.lock().unwrap().clone();
                run_client(ws, frame_rx, config_tx, &initial_state);

                log::info!("WebSocket client disconnected");
            }
            Err(e) => {
                log::warn!("WebSocket handshake failed: {}", e);
            }
        }
    } else {
        // Regular HTTP — serve the UI HTML
        serve_html(stream);
    }
}

fn serve_html(mut stream: TcpStream) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        UI_HTML.len(),
        UI_HTML
    );
    let _ = stream.write_all(response.as_bytes());
}

fn run_broadcast(
    frame_rx: mpsc::Receiver<FrameData>,
    client_reg_rx: mpsc::Receiver<mpsc::SyncSender<Vec<u8>>>,
    running: Arc<AtomicBool>,
) {
    let mut clients: Vec<mpsc::SyncSender<Vec<u8>>> = Vec::new();
    let mut encode_buf = Vec::with_capacity(2048);

    while running.load(Ordering::Relaxed) {
        while let Ok(sender) = client_reg_rx.try_recv() {
            clients.push(sender);
        }

        match frame_rx.recv_timeout(std::time::Duration::from_millis(50)) {
            Ok((leds, mel_bands, beat)) => {
                encode_frame(&leds, &mel_bands, beat, &mut encode_buf);

                clients.retain(|client| {
                    match client.try_send(encode_buf.clone()) {
                        Ok(_) => true,
                        Err(mpsc::TrySendError::Full(_)) => true,
                        Err(mpsc::TrySendError::Disconnected(_)) => false,
                    }
                });
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn build_config_json(
    effect: &str,
    brightness: f32,
    smoothing: f32,
    beat_sensitivity: f32,
) -> String {
    format!(
        r#"{{"type":"state","effect":"{}","brightness":{},"smoothing":{},"beat_sensitivity":{}}}"#,
        effect, brightness, smoothing, beat_sensitivity,
    )
}
