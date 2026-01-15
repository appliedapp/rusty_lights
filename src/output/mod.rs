//! LED output protocols (E1.31, DDP, Art-Net)

mod e131;

#[cfg(feature = "ddp")]
mod ddp;

#[cfg(feature = "artnet")]
mod artnet;

pub use e131::E131Sender;

#[cfg(feature = "ddp")]
pub use ddp::DdpSender;

#[cfg(feature = "artnet")]
pub use artnet::ArtNetSender;

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
