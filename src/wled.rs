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
    leds: WledInfoLeds,
}

#[derive(Debug, Deserialize)]
struct WledInfoLeds {
    count: usize,
}

#[derive(Debug, Deserialize)]
struct WledCfg {
    hw: WledHw,
}

#[derive(Debug, Deserialize)]
struct WledHw {
    led: WledHwLed,
}

#[derive(Debug, Deserialize)]
struct WledHwLed {
    #[serde(default)]
    ins: Vec<WledLedInstance>,
}

#[derive(Debug, Deserialize)]
struct WledLedInstance {
    #[serde(default)]
    order: u8,
}

/// Fetch device info from a WLED controller and merge into `led_config`.
///
/// Only fills in fields that are `None` (not explicitly configured).
/// Returns `true` if any values were auto-detected.
pub fn apply_wled_config(target: &str, led_config: &mut LedConfig) -> bool {
    let mut changed = false;

    if led_config.count.is_none() {
        match fetch_json::<WledInfo>(&format!("http://{target}/json/info")) {
            Ok(info) => {
                log::info!("Auto-detected LED count from WLED: {}", info.leds.count);
                led_config.count = Some(info.leds.count);
                changed = true;
            }
            Err(e) => log::warn!("Could not fetch WLED info from {target}: {e}"),
        }
    }

    if led_config.rgb_order.is_none() {
        match fetch_json::<WledCfg>(&format!("http://{target}/json/cfg")) {
            Ok(cfg) => {
                if let Some(instance) = cfg.hw.led.ins.first() {
                    let order = color_order_from_wled(instance.order);
                    log::info!("Auto-detected RGB order from WLED: {order}");
                    led_config.rgb_order = Some(order.to_string());
                    changed = true;
                }
            }
            Err(e) => log::warn!("Could not fetch WLED config from {target}: {e}"),
        }
    }

    changed
}

fn fetch_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, Box<dyn std::error::Error>> {
    let result: T = ureq::get(url)
        .header("Accept", "application/json")
        .call()?
        .body_mut()
        .read_json()?;
    Ok(result)
}

fn color_order_from_wled(order: u8) -> &'static str {
    match order & 0x0F {
        0 => "GRB",
        1 => "RGB",
        2 => "BRG",
        3 => "RBG",
        4 => "BGR",
        5 => "GBR",
        _ => "GRB",
    }
}
