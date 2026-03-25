// SPDX-License-Identifier: GPL-3.0-only
// Copyright (c) 2026 appliedappliance GmbH
//! Binary frame protocol for WebSocket communication

use crate::effects::Rgb;

/// Serialize frame data into compact binary format.
///
/// Format:
/// - byte 0: flags (bit 0 = beat active)
/// - byte 1: mel band count (N)
/// - bytes 2..2+N: mel bands quantized to 0-255
/// - bytes 2+N..: RGB data (3 bytes per LED)
pub fn encode_frame(leds: &[Rgb], mel_bands: &[f32], beat: Option<f32>, buf: &mut Vec<u8>) {
    let mel_count = mel_bands.len().min(255);
    let total = 2 + mel_count + leds.len() * 3;

    buf.clear();
    buf.reserve(total);

    // Flags
    let flags: u8 = if beat.is_some() { 1 } else { 0 };
    buf.push(flags);

    // Mel band count
    buf.push(mel_count as u8);

    // Mel bands (f32 0.0-1.0 → u8 0-255)
    for &v in &mel_bands[..mel_count] {
        buf.push((v.clamp(0.0, 1.0) * 255.0) as u8);
    }

    // RGB data
    for led in leds {
        buf.push(led.r);
        buf.push(led.g);
        buf.push(led.b);
    }
}
