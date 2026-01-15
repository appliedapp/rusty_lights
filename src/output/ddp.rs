//! DDP (Distributed Display Protocol) implementation

use super::{LedOutput, OutputError};
use crate::effects::Rgb;
use std::net::{SocketAddr, SocketAddrV4, UdpSocket};

/// DDP port
const DDP_PORT: u16 = 4048;

/// DDP header size
const DDP_HEADER_SIZE: usize = 10;

/// DDP sender
pub struct DdpSender {
    socket: UdpSocket,
    target: SocketAddr,
    sequence: u8,
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
        })
    }
}

impl LedOutput for DdpSender {
    fn send(&mut self, leds: &[Rgb]) -> Result<(), OutputError> {
        let data_len = leds.len() * 3;
        let mut packet = vec![0u8; DDP_HEADER_SIZE + data_len];

        // DDP Header
        packet[0] = 0x41; // Flags: VER1 | PUSH
        packet[1] = self.sequence;
        packet[2] = 0x01; // Data type: RGB
        packet[3] = 0x00; // Device ID

        // Data offset (32-bit big-endian)
        packet[4..8].copy_from_slice(&0u32.to_be_bytes());

        // Data length (16-bit big-endian)
        packet[8..10].copy_from_slice(&(data_len as u16).to_be_bytes());

        // Copy RGB data
        for (i, led) in leds.iter().enumerate() {
            let offset = DDP_HEADER_SIZE + i * 3;
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
