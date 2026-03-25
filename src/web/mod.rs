// SPDX-License-Identifier: GPL-3.0-only
// Copyright (c) 2026 appliedappliance GmbH
//! HTTP + WebSocket server for LED digital twin UI

pub mod protocol;
pub mod server;
pub mod websocket;

pub use server::start;
