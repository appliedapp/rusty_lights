// SPDX-License-Identifier: GPL-3.0-only
// Copyright (c) 2026 appliedappliance GmbH
//! DDP (Distributed Display Protocol) implementation
//!
//! DDP is a simple protocol for sending LED data over UDP.
//! It supports fragmentation for large LED counts.

use super::{LedOutput, OutputError};
use crate::effects::Rgb;
use std::net::{SocketAddr, SocketAddrV4, UdpSocket};

/// DDP port
const DDP_PORT: u16 = 4048;

/// DDP header size
const DDP_HEADER_SIZE: usize = 10;

/// Maximum data per packet (typical MTU safe value)
const DDP_MAX_DATA: usize = 1440;

/// DDP flags
mod flags {
    pub const VER1: u8 = 0x40;
    pub const PUSH: u8 = 0x01;
    #[allow(dead_code)]
    pub const TIMECODE: u8 = 0x10;
}

/// DDP data types
mod datatype {
    pub const RGB: u8 = 0x01;
    #[allow(dead_code)]
    pub const RGBW: u8 = 0x02;
}

/// DDP sender
///
/// Supports automatic fragmentation for large LED counts.
pub struct DdpSender {
    socket: UdpSocket,
    target: SocketAddr,
    sequence: u8,
    /// Reusable packet buffer
    packet_buffer: Vec<u8>,
}

impl DdpSender {
    /// Create a new DDP sender
    pub fn new(target: &str) -> Result<Self, OutputError> {
        let ip: std::net::Ipv4Addr = target
            .parse()
            .map_err(|_| OutputError::InvalidAddress(target.to_string()))?;

        let target = SocketAddr::V4(SocketAddrV4::new(ip, DDP_PORT));

        let socket = UdpSocket::bind("0.0.0.0:0")?;

        Ok(Self {
            socket,
            target,
            sequence: 0,
            packet_buffer: vec![0u8; DDP_HEADER_SIZE + DDP_MAX_DATA],
        })
    }

    /// Send a single DDP packet
    fn send_packet(&mut self, data: &[u8], offset: u32, is_last: bool) -> Result<(), OutputError> {
        let data_len = data.len().min(DDP_MAX_DATA);

        // DDP Header
        let mut flags = flags::VER1;
        if is_last {
            flags |= flags::PUSH; // PUSH flag indicates last packet in sequence
        }

        self.packet_buffer[0] = flags;
        self.packet_buffer[1] = self.sequence;
        self.packet_buffer[2] = datatype::RGB;
        self.packet_buffer[3] = 0x00; // Device ID (broadcast)

        // Data offset (32-bit big-endian)
        self.packet_buffer[4..8].copy_from_slice(&offset.to_be_bytes());

        // Data length (16-bit big-endian)
        self.packet_buffer[8..10].copy_from_slice(&(data_len as u16).to_be_bytes());

        // Copy data
        self.packet_buffer[DDP_HEADER_SIZE..DDP_HEADER_SIZE + data_len]
            .copy_from_slice(&data[..data_len]);

        self.socket.send_to(
            &self.packet_buffer[..DDP_HEADER_SIZE + data_len],
            self.target,
        )?;

        Ok(())
    }
}

impl LedOutput for DdpSender {
    fn send(&mut self, leds: &[Rgb]) -> Result<(), OutputError> {
        // Fragment if necessary
        let max_leds_per_packet = DDP_MAX_DATA / 3;
        let num_packets = leds.len().div_ceil(max_leds_per_packet);

        for (packet_idx, chunk) in leds.chunks(max_leds_per_packet).enumerate() {
            let offset = (packet_idx * max_leds_per_packet * 3) as u32;
            let is_last = packet_idx == num_packets - 1;

            // Build RGB data for this chunk
            let chunk_data: Vec<u8> = chunk.iter().flat_map(|led| [led.r, led.g, led.b]).collect();

            self.send_packet(&chunk_data, offset, is_last)?;
        }

        self.sequence = self.sequence.wrapping_add(1);

        Ok(())
    }

    fn target(&self) -> SocketAddr {
        self.target
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ddp_max_leds() {
        // Check that we can handle the maximum LEDs per packet
        let max_leds = DDP_MAX_DATA / 3;
        assert!(
            max_leds >= 400,
            "Should support at least 400 LEDs per packet"
        );
    }
}
