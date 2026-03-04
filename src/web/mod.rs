//! HTTP + WebSocket server for LED digital twin UI

pub mod protocol;
pub mod server;
pub mod websocket;

pub use server::start;
