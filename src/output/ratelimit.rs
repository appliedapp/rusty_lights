// SPDX-License-Identifier: GPL-3.0-only
// Copyright (c) 2026 appliedappliance GmbH
//! Frame rate limiting for consistent output timing

use std::time::{Duration, Instant};

/// Frame rate limiter
///
/// Ensures consistent output timing by sleeping between frames.
pub struct RateLimiter {
    /// Target frame duration
    frame_duration: Duration,
    /// Last frame timestamp
    last_frame: Instant,
    /// Frames sent
    frame_count: u64,
    /// Dropped frames due to overrun
    dropped_frames: u64,
}

impl RateLimiter {
    /// Create a new rate limiter
    ///
    /// # Arguments
    /// * `fps` - Target frames per second (1-240)
    pub fn new(fps: u32) -> Self {
        let fps = fps.clamp(1, 240);
        Self {
            frame_duration: Duration::from_secs_f64(1.0 / fps as f64),
            last_frame: Instant::now(),
            frame_count: 0,
            dropped_frames: 0,
        }
    }

    /// Wait until it's time for the next frame
    ///
    /// Returns `true` if we're on time, `false` if we're behind schedule.
    pub fn wait(&mut self) -> bool {
        let elapsed = self.last_frame.elapsed();

        if elapsed < self.frame_duration {
            // Sleep for remaining time
            let sleep_time = self.frame_duration - elapsed;
            std::thread::sleep(sleep_time);
            self.last_frame = Instant::now();
            self.frame_count += 1;
            true
        } else {
            // We're behind schedule
            let frames_behind = (elapsed.as_secs_f64() / self.frame_duration.as_secs_f64()) as u64;
            if frames_behind > 1 {
                self.dropped_frames += frames_behind - 1;
            }
            self.last_frame = Instant::now();
            self.frame_count += 1;
            false
        }
    }

    /// Reset the timer (call when starting a new session)
    pub fn reset(&mut self) {
        self.last_frame = Instant::now();
        self.frame_count = 0;
        self.dropped_frames = 0;
    }

    /// Set target FPS
    pub fn set_fps(&mut self, fps: u32) {
        let fps = fps.clamp(1, 240);
        self.frame_duration = Duration::from_secs_f64(1.0 / fps as f64);
    }

    /// Get current target FPS
    pub fn fps(&self) -> u32 {
        (1.0 / self.frame_duration.as_secs_f64()).round() as u32
    }

    /// Get total frames sent
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Get number of dropped frames
    pub fn dropped_frames(&self) -> u64 {
        self.dropped_frames
    }

    /// Check if running on time (no recent dropped frames)
    pub fn is_on_time(&self) -> bool {
        self.dropped_frames == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_creation() {
        let limiter = RateLimiter::new(60);
        assert_eq!(limiter.fps(), 60);
    }

    #[test]
    fn test_rate_limiter_bounds() {
        let limiter = RateLimiter::new(0);
        assert_eq!(limiter.fps(), 1);

        let limiter = RateLimiter::new(1000);
        assert_eq!(limiter.fps(), 240);
    }

    #[test]
    fn test_rate_limiter_timing() {
        let mut limiter = RateLimiter::new(100); // 100 FPS = 10ms per frame
        let start = Instant::now();

        // Wait for 5 frames
        for _ in 0..5 {
            limiter.wait();
        }

        let elapsed = start.elapsed();
        // Should take approximately 50ms (5 frames at 10ms each)
        assert!(elapsed.as_millis() >= 40 && elapsed.as_millis() <= 70);
    }
}
