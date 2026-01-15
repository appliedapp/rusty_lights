//! Effect trait and RGB color type

use super::EffectError;

/// RGB color value
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(C)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    /// Create a new RGB color
    #[inline]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Create black (off)
    #[inline]
    pub const fn black() -> Self {
        Self { r: 0, g: 0, b: 0 }
    }

    /// Create white
    #[inline]
    pub const fn white() -> Self {
        Self { r: 255, g: 255, b: 255 }
    }

    /// Create from HSV values
    ///
    /// # Arguments
    /// * `h` - Hue (0.0 - 360.0)
    /// * `s` - Saturation (0.0 - 1.0)
    /// * `v` - Value/Brightness (0.0 - 1.0)
    pub fn from_hsv(h: f32, s: f32, v: f32) -> Self {
        let h = h % 360.0;
        let s = s.clamp(0.0, 1.0);
        let v = v.clamp(0.0, 1.0);

        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;

        let (r, g, b) = match (h / 60.0) as u8 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        Self {
            r: ((r + m) * 255.0) as u8,
            g: ((g + m) * 255.0) as u8,
            b: ((b + m) * 255.0) as u8,
        }
    }

    /// Interpolate between two colors
    #[inline]
    pub fn lerp(a: Rgb, b: Rgb, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let inv_t = 1.0 - t;

        Self {
            r: (a.r as f32 * inv_t + b.r as f32 * t) as u8,
            g: (a.g as f32 * inv_t + b.g as f32 * t) as u8,
            b: (a.b as f32 * inv_t + b.b as f32 * t) as u8,
        }
    }

    /// Scale brightness
    #[inline]
    pub fn scale(self, factor: f32) -> Self {
        let factor = factor.clamp(0.0, 1.0);
        Self {
            r: (self.r as f32 * factor) as u8,
            g: (self.g as f32 * factor) as u8,
            b: (self.b as f32 * factor) as u8,
        }
    }

    /// Apply gamma correction
    #[inline]
    pub fn gamma_correct(self, gamma: f32) -> Self {
        Self {
            r: ((self.r as f32 / 255.0).powf(gamma) * 255.0) as u8,
            g: ((self.g as f32 / 255.0).powf(gamma) * 255.0) as u8,
            b: ((self.b as f32 / 255.0).powf(gamma) * 255.0) as u8,
        }
    }

    /// Convert to byte slice (for protocol output)
    #[inline]
    pub fn as_bytes(&self) -> [u8; 3] {
        [self.r, self.g, self.b]
    }
}

/// LED effect trait
///
/// All effects implement this trait to receive audio data and produce LED colors.
pub trait Effect: Send {
    /// Get the effect name
    fn name(&self) -> &'static str;

    /// Render a single frame
    ///
    /// # Arguments
    /// * `mel_bands` - Mel frequency band energies
    /// * `beat` - Beat detection result (Some(strength) if beat detected)
    /// * `output` - Output LED buffer to write colors to
    fn render(&mut self, mel_bands: &[f32], beat: Option<f32>, output: &mut [Rgb]);

    /// Set a named parameter
    fn set_param(&mut self, name: &str, value: f32) -> Result<(), EffectError>;

    /// Reset effect state
    fn reset(&mut self);
}

/// Pre-defined color gradients
pub struct Gradient {
    colors: Vec<Rgb>,
}

impl Gradient {
    /// Create a rainbow gradient
    pub fn rainbow() -> Self {
        let colors: Vec<Rgb> = (0..256)
            .map(|i| Rgb::from_hsv(i as f32 * 360.0 / 256.0, 1.0, 1.0))
            .collect();
        Self { colors }
    }

    /// Create a fire gradient (red-orange-yellow)
    pub fn fire() -> Self {
        let colors = vec![
            Rgb::new(0, 0, 0),
            Rgb::new(128, 0, 0),
            Rgb::new(255, 0, 0),
            Rgb::new(255, 128, 0),
            Rgb::new(255, 255, 0),
            Rgb::new(255, 255, 128),
        ];
        Self::from_colors(&colors, 256)
    }

    /// Create an ocean gradient (blue-cyan-white)
    pub fn ocean() -> Self {
        let colors = vec![
            Rgb::new(0, 0, 32),
            Rgb::new(0, 0, 128),
            Rgb::new(0, 128, 255),
            Rgb::new(0, 255, 255),
            Rgb::new(128, 255, 255),
        ];
        Self::from_colors(&colors, 256)
    }

    /// Create a gradient from a list of colors
    pub fn from_colors(colors: &[Rgb], steps: usize) -> Self {
        if colors.is_empty() {
            return Self { colors: vec![Rgb::black()] };
        }
        if colors.len() == 1 {
            return Self { colors: vec![colors[0]; steps] };
        }

        let mut result = Vec::with_capacity(steps);
        let segment_size = steps as f32 / (colors.len() - 1) as f32;

        for i in 0..steps {
            let pos = i as f32 / segment_size;
            let idx = pos as usize;
            let t = pos - idx as f32;

            let color = if idx >= colors.len() - 1 {
                colors[colors.len() - 1]
            } else {
                Rgb::lerp(colors[idx], colors[idx + 1], t)
            };

            result.push(color);
        }

        Self { colors: result }
    }

    /// Get color at position (0.0 - 1.0)
    #[inline]
    pub fn get(&self, t: f32) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        let idx = (t * (self.colors.len() - 1) as f32) as usize;
        self.colors[idx.min(self.colors.len() - 1)]
    }

    /// Get color at index
    #[inline]
    pub fn get_index(&self, idx: usize) -> Rgb {
        self.colors[idx % self.colors.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_from_hsv() {
        // Red
        let red = Rgb::from_hsv(0.0, 1.0, 1.0);
        assert_eq!(red.r, 255);
        assert!(red.g < 10);
        assert!(red.b < 10);

        // Green
        let green = Rgb::from_hsv(120.0, 1.0, 1.0);
        assert!(green.r < 10);
        assert_eq!(green.g, 255);
        assert!(green.b < 10);

        // Blue
        let blue = Rgb::from_hsv(240.0, 1.0, 1.0);
        assert!(blue.r < 10);
        assert!(blue.g < 10);
        assert_eq!(blue.b, 255);
    }

    #[test]
    fn test_rgb_lerp() {
        let black = Rgb::black();
        let white = Rgb::white();

        let mid = Rgb::lerp(black, white, 0.5);
        assert!((mid.r as i32 - 127).abs() <= 1);
        assert!((mid.g as i32 - 127).abs() <= 1);
        assert!((mid.b as i32 - 127).abs() <= 1);
    }

    #[test]
    fn test_gradient() {
        let gradient = Gradient::rainbow();

        let start = gradient.get(0.0);
        let end = gradient.get(1.0);

        // Rainbow should start and end with similar hues (red)
        assert!(start.r > 200);
    }
}
