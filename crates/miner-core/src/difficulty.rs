//! Bitcoin difficulty target conversion and utilities.

/// Convert compact "bits" representation to a 256-bit target.
///
/// The bits format is: [exponent (1 byte)][mantissa (3 bytes)]
/// Target = mantissa * 256^(exponent - 3)
///
/// The result is a 32-byte big-endian representation of the target.
pub fn bits_to_target(bits: u32) -> [u8; 32] {
    let exponent = ((bits >> 24) & 0xFF) as usize;
    let mantissa = bits & 0x007FFFFF;

    // Handle negative flag (bit 23 of mantissa)
    // In practice, this shouldn't be set for valid difficulty targets
    let is_negative = (bits & 0x00800000) != 0;
    if is_negative {
        return [0u8; 32]; // Invalid negative target
    }

    let mut target = [0u8; 32];

    if exponent == 0 {
        // Special case: exponent 0 means target is 0
        return target;
    }

    if exponent <= 3 {
        // Mantissa fits in fewer bytes than specified
        let shift = 8 * (3 - exponent);
        let value = mantissa >> shift;

        // Place at the end (least significant position)
        if exponent >= 1 { target[31] = (value & 0xFF) as u8; }
        if exponent >= 2 { target[30] = ((value >> 8) & 0xFF) as u8; }
        if exponent >= 3 { target[29] = ((value >> 16) & 0xFF) as u8; }
    } else {
        // Normal case: mantissa goes at position (32 - exponent)
        let pos = 32 - exponent;

        // Place the 3 mantissa bytes
        if pos < 32 { target[pos] = ((mantissa >> 16) & 0xFF) as u8; }
        if pos + 1 < 32 { target[pos + 1] = ((mantissa >> 8) & 0xFF) as u8; }
        if pos + 2 < 32 { target[pos + 2] = (mantissa & 0xFF) as u8; }
    }

    target
}

/// Convert a 256-bit target back to compact "bits" representation.
///
/// This is the inverse of `bits_to_target`.
pub fn target_to_bits(target: &[u8; 32]) -> u32 {
    // Find the first non-zero byte
    let mut first_nonzero = 0;
    while first_nonzero < 32 && target[first_nonzero] == 0 {
        first_nonzero += 1;
    }

    if first_nonzero == 32 {
        // All zeros - return 0
        return 0;
    }

    // Calculate exponent (number of bytes from the right)
    let exponent = (32 - first_nonzero) as u32;

    // Extract mantissa (up to 3 bytes starting at first non-zero)
    let mut mantissa: u32 = 0;

    if first_nonzero < 32 {
        mantissa |= (target[first_nonzero] as u32) << 16;
    }
    if first_nonzero + 1 < 32 {
        mantissa |= (target[first_nonzero + 1] as u32) << 8;
    }
    if first_nonzero + 2 < 32 {
        mantissa |= target[first_nonzero + 2] as u32;
    }

    // If the high bit of mantissa is set, we need to shift right
    // to avoid the negative flag
    let (exp_adj, mant_adj) = if mantissa & 0x00800000 != 0 {
        (exponent + 1, mantissa >> 8)
    } else {
        (exponent, mantissa)
    };

    (exp_adj << 24) | (mant_adj & 0x007FFFFF)
}

/// Check if a hash meets the difficulty target.
///
/// Returns true if hash < target (valid proof of work).
#[inline]
pub fn hash_meets_target(hash: &[u8; 32], target: &[u8; 32]) -> bool {
    // Both are 32-byte big-endian numbers
    // Compare byte by byte from most significant
    for i in 0..32 {
        if hash[i] < target[i] {
            return true;
        }
        if hash[i] > target[i] {
            return false;
        }
    }
    // Exactly equal - technically meets target but extremely rare
    true
}

/// Calculate approximate difficulty from bits.
///
/// Difficulty = max_target / current_target
/// Where max_target is the genesis block target (bits = 0x1d00ffff)
pub fn bits_to_difficulty(bits: u32) -> f64 {
    // Genesis block bits: 0x1d00ffff
    // This represents the "difficulty 1" target
    const GENESIS_BITS: u32 = 0x1d00ffff;

    let current_target = bits_to_target(bits);
    let genesis_target = bits_to_target(GENESIS_BITS);

    // Convert to f64 for division (approximate)
    let current_f64 = target_to_f64(&current_target);
    let genesis_f64 = target_to_f64(&genesis_target);

    if current_f64 == 0.0 {
        return f64::INFINITY;
    }

    genesis_f64 / current_f64
}

/// Convert a 256-bit target to an approximate f64 value.
fn target_to_f64(target: &[u8; 32]) -> f64 {
    // Find first non-zero byte
    let mut first_nonzero = 0;
    while first_nonzero < 32 && target[first_nonzero] == 0 {
        first_nonzero += 1;
    }

    if first_nonzero == 32 {
        return 0.0;
    }

    // Take up to 8 bytes for precision
    let mut value: u64 = 0;
    for i in 0..8 {
        if first_nonzero + i < 32 {
            value = (value << 8) | (target[first_nonzero + i] as u64);
        }
    }

    // Scale by position using libm for no_std compatibility
    let shift = (31 - first_nonzero) * 8;
    let exponent = (shift as i32) - 56;
    (value as f64) * pow2_f64(exponent)
}

/// Compute 2^exp for f64, no_std compatible.
fn pow2_f64(exp: i32) -> f64 {
    if exp >= 0 {
        (1u64 << exp.min(63)) as f64
    } else {
        1.0 / (1u64 << (-exp).min(63)) as f64
    }
}

/// Format difficulty for display (e.g., "1.23T" for trillion).
pub fn format_difficulty(difficulty: f64) -> alloc::string::String {
    if difficulty >= 1e15 {
        alloc::format!("{:.2}P", difficulty / 1e15)
    } else if difficulty >= 1e12 {
        alloc::format!("{:.2}T", difficulty / 1e12)
    } else if difficulty >= 1e9 {
        alloc::format!("{:.2}G", difficulty / 1e9)
    } else if difficulty >= 1e6 {
        alloc::format!("{:.2}M", difficulty / 1e6)
    } else if difficulty >= 1e3 {
        alloc::format!("{:.2}K", difficulty / 1e3)
    } else {
        alloc::format!("{:.2}", difficulty)
    }
}

/// Estimate average hashes needed to find a block at given difficulty.
pub fn expected_hashes(difficulty: f64) -> f64 {
    // On average, need difficulty * 2^32 hashes
    difficulty * 4_294_967_296.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bits_to_target_genesis() {
        // Genesis block bits: 0x1d00ffff
        let bits = 0x1d00ffff;
        let target = bits_to_target(bits);

        // Expected target starts with 00000000ffff...
        assert_eq!(target[0], 0x00);
        assert_eq!(target[1], 0x00);
        assert_eq!(target[2], 0x00);
        assert_eq!(target[3], 0x00);
        assert_eq!(target[4], 0xff);
        assert_eq!(target[5], 0xff);

        // Rest should be zeros
        for i in 6..32 {
            assert_eq!(target[i], 0x00, "byte {} should be 0", i);
        }
    }

    #[test]
    fn test_bits_to_target_high_difficulty() {
        // A more recent high-difficulty bits value
        // bits = 0x17034219 (example from a recent block)
        let bits = 0x17034219;
        let target = bits_to_target(bits);

        // Exponent = 0x17 = 23, so target starts at byte 32-23 = 9
        // First 9 bytes should be zero
        for i in 0..9 {
            assert_eq!(target[i], 0x00, "byte {} should be 0", i);
        }

        // Mantissa 0x034219 should be at bytes 9, 10, 11
        assert_eq!(target[9], 0x03);
        assert_eq!(target[10], 0x42);
        assert_eq!(target[11], 0x19);
    }

    #[test]
    fn test_bits_roundtrip() {
        let test_cases = [
            0x1d00ffff, // Genesis
            0x17034219, // High difficulty
            0x1b0404cb, // Medium difficulty
        ];

        for &bits in &test_cases {
            let target = bits_to_target(bits);
            let recovered = target_to_bits(&target);
            assert_eq!(bits, recovered, "Roundtrip failed for bits {:08x}", bits);
        }
    }

    #[test]
    fn test_hash_meets_target() {
        let target = bits_to_target(0x1d00ffff);

        // A hash with 4 leading zero bytes should pass genesis difficulty
        let good_hash = [
            0x00, 0x00, 0x00, 0x00, 0x12, 0x34, 0x56, 0x78,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        assert!(hash_meets_target(&good_hash, &target));

        // A hash without enough leading zeros should fail
        let bad_hash = [
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        assert!(!hash_meets_target(&bad_hash, &target));
    }

    #[test]
    fn test_difficulty_calculation() {
        // Genesis block should have difficulty 1
        let genesis_diff = bits_to_difficulty(0x1d00ffff);
        assert!((genesis_diff - 1.0).abs() < 0.01);
    }
}
