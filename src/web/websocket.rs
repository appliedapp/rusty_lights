// SPDX-License-Identifier: GPL-3.0-only
// Copyright (c) 2026 appliedappliance GmbH
//! Per-client WebSocket read/write loop

use std::net::TcpStream;
use std::sync::mpsc;

use tungstenite::WebSocket;
use tungstenite::protocol::Message;

use crate::engine::ConfigUpdateMsg;

/// Run a WebSocket client loop.
///
/// Reads incoming text messages (config changes) and forwards them.
/// Receives frame data on `frame_rx` and sends binary messages.
/// Exits when frame channel closes or WebSocket errors.
pub fn run_client(
    mut ws: WebSocket<&TcpStream>,
    frame_rx: mpsc::Receiver<Vec<u8>>,
    config_tx: mpsc::Sender<ConfigUpdateMsg>,
    initial_state: &str,
) {
    // Send initial config state
    if ws.send(Message::Text(initial_state.into())).is_err() {
        return;
    }

    // Set non-blocking so we can interleave reads and frame sends
    if let Ok(stream) = ws.get_ref().try_clone() {
        let _ = stream.set_nonblocking(true);
    }

    loop {
        // Try to read incoming messages (non-blocking)
        match ws.read() {
            Ok(Message::Text(text)) => {
                handle_incoming_message(&text, &config_tx);
            }
            Ok(Message::Close(_)) => break,
            Ok(Message::Ping(data)) => {
                let _ = ws.send(Message::Pong(data));
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(ref e)) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => break,
        }

        // Try to receive and send a frame
        match frame_rx.recv_timeout(std::time::Duration::from_millis(16)) {
            Ok(data) => {
                if ws.send(Message::Binary(data.into())).is_err() {
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = ws.close(None);
}

fn handle_incoming_message(text: &str, config_tx: &mpsc::Sender<ConfigUpdateMsg>) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return;
    };

    if let Some(effect) = value.get("effect").and_then(|v| v.as_str()) {
        let _ = config_tx.send(ConfigUpdateMsg::SetEffect(effect.to_string()));
    }
    if let Some(brightness) = value.get("brightness").and_then(|v| v.as_f64()) {
        let _ = config_tx.send(ConfigUpdateMsg::SetBrightness(brightness as f32));
    }
    if let Some(smoothing) = value.get("smoothing").and_then(|v| v.as_f64()) {
        let _ = config_tx.send(ConfigUpdateMsg::SetSmoothing(smoothing as f32));
    }
    if let Some(sensitivity) = value.get("beat_sensitivity").and_then(|v| v.as_f64()) {
        let _ = config_tx.send(ConfigUpdateMsg::SetBeatSensitivity(sensitivity as f32));
    }
}
