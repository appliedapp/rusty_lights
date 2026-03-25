// SPDX-License-Identifier: GPL-3.0-only
// Copyright (c) 2026 appliedappliance GmbH
//! Art-Net protocol implementation (placeholder)

use super::{LedOutput, OutputError};
use crate::effects::Rgb;
use std::net::{SocketAddr, SocketAddrV4, UdpSocket};

/// Art-Net port
const ARTNET_PORT: u16 = 6454;

/// Art-Net sender (placeholder implementation)
pub struct ArtNetSender {
    socket: UdpSocket,
    target: SocketAddr,
    sequence: u8,
    universe: u16,
}

impl ArtNetSender {
    /// Create a new Art-Net sender
    pub fn new(target: &str, universe: u16) -> Result<Self, OutputError> {
        let ip: std::net::Ipv4Addr = target
            .parse()
            .map_err(|_| OutputError::InvalidAddress(target.to_string()))?;

        let target = SocketAddr::V4(SocketAddrV4::new(ip, ARTNET_PORT));

        let socket = UdpSocket::bind("0.0.0.0:0")?;

        Ok(Self {
            socket,
            target,
            sequence: 0,
            universe,
        })
    }
}

impl LedOutput for ArtNetSender {
    fn send(&mut self, leds: &[Rgb]) -> Result<(), OutputError> {
        // Art-Net DMX packet (ArtDmx)
        let data_len = (leds.len() * 3).min(512);
        let mut packet = vec![0u8; 18 + data_len];

        // Art-Net header
        packet[0..8].copy_from_slice(b"Art-Net\x00");

        // OpCode: OpDmx (0x5000)
        packet[8..10].copy_from_slice(&0x0050u16.to_le_bytes());

        // Protocol version (14)
        packet[10] = 0x00;
        packet[11] = 14;

        // Sequence
        packet[12] = self.sequence;

        // Physical port
        packet[13] = 0;

        // Universe (low byte first)
        packet[14..16].copy_from_slice(&self.universe.to_le_bytes());

        // Data length (high byte first)
        packet[16..18].copy_from_slice(&(data_len as u16).to_be_bytes());

        // Copy RGB data
        for (i, led) in leds.iter().enumerate().take(data_len / 3) {
            let offset = 18 + i * 3;
            packet[offset] = led.r;
            packet[offset + 1] = led.g;
            packet[offset + 2] = led.b;
        }

        self.socket.send_to(&packet, self.target)?;

        self.sequence = self.sequence.wrapping_add(1);

        Ok(())
    }

    fn target(&self) -> SocketAddr {
        self.target
    }
}
