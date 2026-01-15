//! Utility functions and SIMD helpers

#[cfg(feature = "simd")]
pub mod simd;

/// Convert RGB byte order
pub fn reorder_rgb(data: &mut [u8], order: &str) {
    match order.to_uppercase().as_str() {
        "RGB" => {} // No change needed
        "GRB" => {
            for chunk in data.chunks_exact_mut(3) {
                chunk.swap(0, 1);
            }
        }
        "BGR" => {
            for chunk in data.chunks_exact_mut(3) {
                chunk.swap(0, 2);
            }
        }
        "RBG" => {
            for chunk in data.chunks_exact_mut(3) {
                chunk.swap(1, 2);
            }
        }
        "BRG" => {
            for chunk in data.chunks_exact_mut(3) {
                let tmp = chunk[0];
                chunk[0] = chunk[2];
                chunk[2] = chunk[1];
                chunk[1] = tmp;
            }
        }
        "GBR" => {
            for chunk in data.chunks_exact_mut(3) {
                let tmp = chunk[0];
                chunk[0] = chunk[1];
                chunk[1] = chunk[2];
                chunk[2] = tmp;
            }
        }
        _ => {} // Unknown order, leave unchanged
    }
}

/// Pre-computed gamma correction lookup table (gamma = 2.2)
/// Generated at compile time
pub const GAMMA_LUT: [u8; 256] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2,
    2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 6, 6, 6,
    7, 7, 8, 8, 8, 9, 9, 10, 10, 10, 11, 11, 12, 12, 13, 13,
    14, 14, 15, 15, 16, 16, 17, 17, 18, 18, 19, 19, 20, 21, 21, 22,
    22, 23, 24, 24, 25, 26, 26, 27, 28, 28, 29, 30, 30, 31, 32, 32,
    33, 34, 35, 35, 36, 37, 38, 38, 39, 40, 41, 41, 42, 43, 44, 45,
    46, 46, 47, 48, 49, 50, 51, 52, 53, 53, 54, 55, 56, 57, 58, 59,
    60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75,
    76, 77, 78, 79, 80, 81, 82, 84, 85, 86, 87, 88, 89, 90, 92, 93,
    94, 95, 96, 98, 99, 100, 101, 103, 104, 105, 106, 108, 109, 110, 111, 113,
    114, 115, 117, 118, 119, 121, 122, 123, 125, 126, 127, 129, 130, 131, 133, 134,
    136, 137, 138, 140, 141, 143, 144, 146, 147, 149, 150, 152, 153, 155, 156, 158,
    159, 161, 162, 164, 165, 167, 168, 170, 171, 173, 175, 176, 178, 179, 181, 183,
    184, 186, 188, 189, 191, 193, 194, 196, 198, 199, 201, 203, 204, 206, 208, 209,
    211, 213, 215, 216, 218, 220, 222, 223, 225, 227, 229, 231, 232, 234, 236, 255,
];

/// Apply gamma correction using lookup table
#[inline]
pub fn gamma_correct(value: u8) -> u8 {
    GAMMA_LUT[value as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reorder_grb() {
        let mut data = [1, 2, 3, 4, 5, 6];
        reorder_rgb(&mut data, "GRB");
        assert_eq!(data, [2, 1, 3, 5, 4, 6]);
    }

    #[test]
    fn test_reorder_bgr() {
        let mut data = [1, 2, 3];
        reorder_rgb(&mut data, "BGR");
        assert_eq!(data, [3, 2, 1]);
    }

    #[test]
    fn test_gamma_lut() {
        assert_eq!(GAMMA_LUT[0], 0);
        assert_eq!(GAMMA_LUT[255], 255);
        // Mid values should be lower due to gamma
        assert!(GAMMA_LUT[128] < 128);
    }
}
