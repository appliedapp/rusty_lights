// SPDX-License-Identifier: GPL-3.0-only
// Copyright (c) 2026 appliedappliance GmbH
//! WLED device auto-configuration
//!
//! Fetches LED configuration from a WLED device's JSON API and fills in
//! any values not explicitly set in the local config.

use crate::config::LedConfig;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct WledInfo {
    leds: WledLeds,
}

#[derive(Debug, Deserialize)]
struct WledLeds {
    count: usize,
}

/// Fetch device info from a WLED controller and merge into `led_config`.
///
/// Only fills in fields that are `None` (not explicitly configured).
/// Returns `true` if any values were auto-detected.
pub fn apply_wled_config(target: &str, led_config: &mut LedConfig) -> bool {
    let url = format!("http://{target}/json/info");

    let info = match fetch_info(&url) {
        Ok(info) => info,
        Err(e) => {
            log::warn!("Could not fetch WLED config from {target}: {e}");
            return false;
        }
    };

    let mut changed = false;

    if led_config.count.is_none() {
        log::info!("Auto-detected LED count from WLED: {}", info.leds.count);
        led_config.count = Some(info.leds.count);
        changed = true;
    }

    changed
}

fn fetch_info(url: &str) -> Result<WledInfo, Box<dyn std::error::Error>> {
    let info: WledInfo = ureq::get(url)
        .header("Accept", "application/json")
        .call()?
        .body_mut()
        .read_json()?;
    Ok(info)
}
