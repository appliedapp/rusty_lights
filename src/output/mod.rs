//! LED output protocols (E1.31, DDP, Art-Net)
//!
//! Provides multiple protocols for sending LED data:
//! - **E1.31/sACN**: Industry standard, multicast support
//! - **DDP**: Simple protocol, good for WLED
//! - **Art-Net**: DMX over IP standard

mod e131;
mod ddp;
mod artnet;
mod ratelimit;

pub use e131::E131Sender;
pub use ddp::DdpSender;
pub use artnet::ArtNetSender;
pub use ratelimit::RateLimiter;

use crate::effects::Rgb;
use std::net::SocketAddr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OutputError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
    #[error("Send failed: {0}")]
    SendFailed(String),
    #[error("Unknown protocol: {0}")]
    UnknownProtocol(String),
}

/// Common trait for LED output protocols
pub trait LedOutput: Send {
    /// Send LED data
    fn send(&mut self, leds: &[Rgb]) -> Result<(), OutputError>;

    /// Get target address
    fn target(&self) -> SocketAddr;
}

/// Multi-universe output handler
///
/// Automatically splits LED data across multiple universes.
pub struct MultiUniverseOutput {
    senders: Vec<E131Sender>,
    base_universe: u16,
    leds_per_universe: usize,
}

impl MultiUniverseOutput {
    /// Create a new multi-universe output
    ///
    /// # Arguments
    /// * `target` - Target address (multicast or unicast)
    /// * `base_universe` - Starting universe number
    /// * `num_leds` - Total number of LEDs
    pub fn new(target: &str, base_universe: u16, num_leds: usize) -> Result<Self, OutputError> {
        // 170 RGB LEDs per universe (510 channels, max 512)
        let leds_per_universe = 170;
        let num_universes = (num_leds + leds_per_universe - 1) / leds_per_universe;

        let mut senders = Vec::with_capacity(num_universes);
        for i in 0..num_universes {
            let universe = base_universe + i as u16;
            let sender = E131Sender::new(target, universe)?;
            senders.push(sender);
        }

        Ok(Self {
            senders,
            base_universe,
            leds_per_universe,
        })
    }

    /// Send LED data across all universes
    pub fn send(&mut self, leds: &[Rgb]) -> Result<(), OutputError> {
        for (i, chunk) in leds.chunks(self.leds_per_universe).enumerate() {
            if i < self.senders.len() {
                self.senders[i].send(chunk)?;
            }
        }
        Ok(())
    }

    /// Get the base universe
    pub fn base_universe(&self) -> u16 {
        self.base_universe
    }

    /// Get the number of universes
    pub fn num_universes(&self) -> usize {
        self.senders.len()
    }
}

/// Create an output sender by protocol name
///
/// # Arguments
/// * `protocol` - Protocol name: "e131", "ddp", or "artnet"
/// * `target` - Target address (IP or "multicast" for E1.31)
/// * `universe` - Universe number (for E1.31 and Art-Net)
pub fn create_output(
    protocol: &str,
    target: &str,
    universe: u16,
) -> Result<Box<dyn LedOutput>, OutputError> {
    match protocol.to_lowercase().as_str() {
        "e131" | "sacn" => {
            let sender = E131Sender::new(target, universe)?;
            Ok(Box::new(sender))
        }
        "ddp" => {
            let sender = DdpSender::new(target)?;
            Ok(Box::new(sender))
        }
        "artnet" | "art-net" => {
            let sender = ArtNetSender::new(target, universe)?;
            Ok(Box::new(sender))
        }
        _ => Err(OutputError::UnknownProtocol(protocol.to_string())),
    }
}

/// List available output protocols
pub fn available_protocols() -> &'static [&'static str] {
    &["e131", "ddp", "artnet"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_available_protocols() {
        let protocols = available_protocols();
        assert!(protocols.contains(&"e131"));
        assert!(protocols.contains(&"ddp"));
        assert!(protocols.contains(&"artnet"));
    }

    #[test]
    fn test_create_output_e131() {
        // Use unicast to avoid multicast setup issues in tests
        let output = create_output("e131", "127.0.0.1", 1);
        assert!(output.is_ok());
    }

    #[test]
    fn test_create_output_ddp() {
        let output = create_output("ddp", "127.0.0.1", 1);
        assert!(output.is_ok());
    }

    #[test]
    fn test_create_output_artnet() {
        let output = create_output("artnet", "127.0.0.1", 1);
        assert!(output.is_ok());
    }

    #[test]
    fn test_create_output_unknown() {
        let output = create_output("unknown", "127.0.0.1", 1);
        assert!(output.is_err());
    }

    #[test]
    fn test_multi_universe() {
        // 200 LEDs should require 2 universes (170 LEDs per universe)
        let output = MultiUniverseOutput::new("127.0.0.1", 1, 200);
        assert!(output.is_ok());
        let output = output.unwrap();
        assert_eq!(output.num_universes(), 2);
        assert_eq!(output.base_universe(), 1);
    }

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(60);
        assert_eq!(limiter.fps(), 60);
        assert_eq!(limiter.frame_count(), 0);
    }
}
