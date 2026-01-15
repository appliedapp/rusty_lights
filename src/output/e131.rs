//! E1.31 (sACN) protocol implementation

use super::{LedOutput, OutputError};
use crate::effects::Rgb;
use socket2::{Domain, Protocol, Socket, Type};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use uuid::Uuid;

/// E1.31 packet header size
const E131_HEADER_SIZE: usize = 126;

/// Maximum DMX data size
const DMX_MAX_SIZE: usize = 512;

/// E1.31 port
const E131_PORT: u16 = 5568;

/// E1.31/sACN sender
pub struct E131Sender {
    socket: UdpSocket,
    target: SocketAddr,
    packet_buffer: Box<[u8; E131_HEADER_SIZE + DMX_MAX_SIZE]>,
    sequence: u8,
    universe: u16,
}

impl E131Sender {
    /// Create a new E1.31 sender
    ///
    /// # Arguments
    /// * `target` - Target address (IP or "multicast" for auto)
    /// * `universe` - DMX universe (1-63999)
    pub fn new(target: &str, universe: u16) -> Result<Self, OutputError> {
        let target_addr = Self::resolve_target(target, universe)?;

        // Create UDP socket
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;

        // Enable multicast if target is multicast address
        if target_addr.ip().is_multicast() {
            socket.set_multicast_ttl_v4(20)?;
        }

        // Allow address reuse
        socket.set_reuse_address(true)?;

        // Bind to any available port
        socket.bind(&SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0).into())?;

        let socket: UdpSocket = socket.into();

        let mut sender = Self {
            socket,
            target: target_addr,
            packet_buffer: Box::new([0u8; E131_HEADER_SIZE + DMX_MAX_SIZE]),
            sequence: 0,
            universe,
        };

        sender.init_packet_header();

        Ok(sender)
    }

    /// Resolve target address
    fn resolve_target(target: &str, universe: u16) -> Result<SocketAddr, OutputError> {
        if target == "multicast" || target.starts_with("239.255.") {
            // E1.31 multicast address: 239.255.{high}.{low}
            let addr = Self::universe_to_multicast(universe);
            Ok(SocketAddr::V4(SocketAddrV4::new(addr, E131_PORT)))
        } else {
            // Parse as unicast IP
            let ip: Ipv4Addr = target
                .parse()
                .map_err(|_| OutputError::InvalidAddress(target.to_string()))?;
            Ok(SocketAddr::V4(SocketAddrV4::new(ip, E131_PORT)))
        }
    }

    /// Convert universe to multicast address
    #[inline]
    fn universe_to_multicast(universe: u16) -> Ipv4Addr {
        Ipv4Addr::new(239, 255, (universe >> 8) as u8, universe as u8)
    }

    /// Initialize static packet header fields
    fn init_packet_header(&mut self) {
        let buf = &mut self.packet_buffer;
        let cid = Uuid::new_v4();

        // Root Layer
        buf[0..2].copy_from_slice(&0x0010u16.to_be_bytes()); // Preamble size
        buf[2..4].copy_from_slice(&0x0000u16.to_be_bytes()); // Postamble size
        buf[4..16].copy_from_slice(b"ASC-E1.17\x00\x00\x00"); // ACN Packet ID

        // Framing Layer Vector
        buf[18..22].copy_from_slice(&0x00000004u32.to_be_bytes()); // Root Vector

        // CID (Component Identifier)
        buf[22..38].copy_from_slice(cid.as_bytes());

        // Framing Vector
        buf[40..44].copy_from_slice(&0x00000002u32.to_be_bytes());

        // Source name (64 bytes, null-padded)
        let source_name = b"RustyLights";
        buf[44..44 + source_name.len()].copy_from_slice(source_name);

        // Priority
        buf[108] = 100;

        // Sync address (not used)
        buf[109..111].copy_from_slice(&0x0000u16.to_be_bytes());

        // Options
        buf[112] = 0x00;

        // Universe
        buf[113..115].copy_from_slice(&self.universe.to_be_bytes());

        // DMP Layer
        buf[117] = 0x02; // DMP Vector
        buf[118] = 0xA1; // Address type & data type
        buf[119..121].copy_from_slice(&0x0000u16.to_be_bytes()); // First property address
        buf[121..123].copy_from_slice(&0x0001u16.to_be_bytes()); // Address increment

        // Start code (DMX512)
        buf[125] = 0x00;
    }

    /// Update packet with LED data and send
    fn send_packet(&mut self, dmx_data: &[u8]) -> Result<(), OutputError> {
        let data_len = dmx_data.len().min(DMX_MAX_SIZE);
        let packet_len = E131_HEADER_SIZE + data_len;

        // Update sequence number
        self.sequence = self.sequence.wrapping_add(1);
        self.packet_buffer[111] = self.sequence;

        // Update length fields
        let root_length = (packet_len - 16) as u16;
        let frame_length = (packet_len - 38) as u16;
        let dmp_length = (packet_len - 115) as u16;
        let property_count = (data_len + 1) as u16; // +1 for start code

        self.packet_buffer[16..18].copy_from_slice(&(0x7000 | root_length).to_be_bytes());
        self.packet_buffer[38..40].copy_from_slice(&(0x7000 | frame_length).to_be_bytes());
        self.packet_buffer[115..117].copy_from_slice(&(0x7000 | dmp_length).to_be_bytes());
        self.packet_buffer[123..125].copy_from_slice(&property_count.to_be_bytes());

        // Copy DMX data
        self.packet_buffer[126..126 + data_len].copy_from_slice(&dmx_data[..data_len]);

        // Send packet
        self.socket
            .send_to(&self.packet_buffer[..packet_len], self.target)?;

        Ok(())
    }

    /// Get the current universe
    pub fn universe(&self) -> u16 {
        self.universe
    }
}

impl LedOutput for E131Sender {
    fn send(&mut self, leds: &[Rgb]) -> Result<(), OutputError> {
        // Convert RGB to DMX data (max 170 LEDs = 510 channels)
        let max_leds = DMX_MAX_SIZE / 3;
        let num_leds = leds.len().min(max_leds);

        let mut dmx_data = [0u8; DMX_MAX_SIZE];
        for (i, led) in leds.iter().enumerate().take(num_leds) {
            let offset = i * 3;
            dmx_data[offset] = led.r;
            dmx_data[offset + 1] = led.g;
            dmx_data[offset + 2] = led.b;
        }

        self.send_packet(&dmx_data[..num_leds * 3])
    }

    fn target(&self) -> SocketAddr {
        self.target
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multicast_address() {
        assert_eq!(
            E131Sender::universe_to_multicast(1),
            Ipv4Addr::new(239, 255, 0, 1)
        );
        assert_eq!(
            E131Sender::universe_to_multicast(256),
            Ipv4Addr::new(239, 255, 1, 0)
        );
        assert_eq!(
            E131Sender::universe_to_multicast(63999),
            Ipv4Addr::new(239, 255, 249, 255)
        );
    }
}
