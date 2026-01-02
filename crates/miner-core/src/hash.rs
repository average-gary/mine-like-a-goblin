//! SHA256 double-hashing and mining functions.

use sha2::{Digest, Sha256};

/// Bitcoin's double SHA256: SHA256(SHA256(data)).
///
/// This is used for block header hashing, transaction IDs, and merkle trees.
#[inline]
pub fn double_sha256(data: &[u8]) -> [u8; 32] {
    let first = Sha256::digest(data);
    let second = Sha256::digest(&first);
    let mut result = [0u8; 32];
    result.copy_from_slice(&second);
    result
}

/// Single SHA256 hash.
#[inline]
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let hash = Sha256::digest(data);
    let mut result = [0u8; 32];
    result.copy_from_slice(&hash);
    result
}

/// Result of a mining batch operation.
#[derive(Debug, Clone)]
pub struct MiningResult {
    /// The nonce that produced the hash (if found).
    pub nonce: Option<u32>,
    /// The resulting block hash (if found).
    pub hash: Option<[u8; 32]>,
    /// Number of hashes computed in this batch.
    pub hashes_computed: u64,
    /// Whether a share (lower difficulty) was found.
    pub share_found: bool,
    /// Whether a valid block was found.
    pub block_found: bool,
}

impl MiningResult {
    /// Create a result indicating no match found.
    pub fn not_found(hashes: u64) -> Self {
        MiningResult {
            nonce: None,
            hash: None,
            hashes_computed: hashes,
            share_found: false,
            block_found: false,
        }
    }

    /// Create a result indicating a share was found.
    pub fn share(nonce: u32, hash: [u8; 32], hashes: u64) -> Self {
        MiningResult {
            nonce: Some(nonce),
            hash: Some(hash),
            hashes_computed: hashes,
            share_found: true,
            block_found: false,
        }
    }

    /// Create a result indicating a valid block was found.
    pub fn block(nonce: u32, hash: [u8; 32], hashes: u64) -> Self {
        MiningResult {
            nonce: Some(nonce),
            hash: Some(hash),
            hashes_computed: hashes,
            share_found: true,
            block_found: true,
        }
    }
}

/// Mine a range of nonces, checking for both shares and valid blocks.
///
/// # Arguments
/// * `header_without_nonce` - 76-byte block header (everything except the nonce)
/// * `block_target` - 32-byte target that block hash must be below
/// * `share_min_zeros` - Minimum leading zero bits (in display format) for a share
/// * `nonce_start` - Starting nonce value
/// * `nonce_count` - Number of nonces to try
///
/// # Returns
/// A `MiningResult` indicating what was found (if anything).
pub fn mine_batch(
    header_without_nonce: &[u8; 76],
    block_target: &[u8; 32],
    share_min_zeros: u32,
    nonce_start: u32,
    nonce_count: u32,
) -> MiningResult {
    // Pre-allocate the full 80-byte header
    let mut header = [0u8; 80];
    header[..76].copy_from_slice(header_without_nonce);

    let nonce_end = nonce_start.saturating_add(nonce_count);
    let mut best_share: Option<(u32, [u8; 32], u32)> = None; // (nonce, hash, leading_zeros)

    for nonce in nonce_start..nonce_end {
        // Set the nonce (little-endian at bytes 76-79)
        header[76..80].copy_from_slice(&nonce.to_le_bytes());

        // Compute double SHA256
        let hash = double_sha256(&header);

        // Check if hash meets block target (valid block!)
        if hash_below_target(&hash, block_target) {
            return MiningResult::block(nonce, hash, (nonce - nonce_start + 1) as u64);
        }

        // Check if hash qualifies as a share (enough leading zeros in display format)
        let leading_zeros = count_leading_zeros(&hash);
        if leading_zeros >= share_min_zeros {
            // Keep track of the best (most leading zeros) share found
            match &best_share {
                None => best_share = Some((nonce, hash, leading_zeros)),
                Some((_, _, best_zeros)) if leading_zeros > *best_zeros => {
                    best_share = Some((nonce, hash, leading_zeros));
                }
                _ => {}
            }
        }
    }

    let hashes = nonce_count as u64;

    // Return share if found, otherwise no result
    if let Some((nonce, hash, _)) = best_share {
        MiningResult::share(nonce, hash, hashes)
    } else {
        MiningResult::not_found(hashes)
    }
}

/// Check if a hash is below a target (valid proof of work).
///
/// Both hash and target are treated as 256-bit big-endian numbers.
/// The hash must be strictly less than the target.
#[inline]
pub fn hash_below_target(hash: &[u8; 32], target: &[u8; 32]) -> bool {
    // Compare as big-endian: start from most significant byte
    for i in 0..32 {
        if hash[i] < target[i] {
            return true;
        }
        if hash[i] > target[i] {
            return false;
        }
    }
    // Equal - not below
    false
}

/// Reverse the byte order of a 32-byte array.
///
/// Bitcoin often displays hashes in reverse byte order (little-endian display).
#[inline]
pub fn reverse_bytes(bytes: &[u8; 32]) -> [u8; 32] {
    let mut reversed = [0u8; 32];
    for i in 0..32 {
        reversed[i] = bytes[31 - i];
    }
    reversed
}

/// Convert a hash to its display format (reversed hex).
pub fn hash_to_display_hex(hash: &[u8; 32]) -> alloc::string::String {
    let reversed = reverse_bytes(hash);
    hex::encode(reversed)
}

/// Count leading zero bits in the DISPLAYED hash format.
///
/// Bitcoin hashes are displayed in reversed byte order, so the "leading zeros"
/// you see in a block hash like "00000000000..." correspond to the TRAILING
/// bytes of the internal hash representation.
///
/// This function counts zeros from the end of the internal array, which
/// corresponds to the beginning of the displayed hash.
pub fn count_leading_zeros(hash: &[u8; 32]) -> u32 {
    let mut zeros = 0u32;
    // Iterate in reverse - displayed hash is byte-reversed
    for byte in hash.iter().rev() {
        if *byte == 0 {
            zeros += 8;
        } else {
            zeros += byte.leading_zeros();
            break;
        }
    }
    zeros
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_double_sha256() {
        // Test vector: SHA256d("hello")
        let data = b"hello";
        let hash = double_sha256(data);

        // Known result for double SHA256 of "hello"
        let expected = hex::decode(
            "9595c9df90075148eb06860365df33584b75bff782a510c6cd4883a419833d50"
        ).unwrap();

        assert_eq!(hash.as_slice(), expected.as_slice());
    }

    #[test]
    fn test_hash_below_target() {
        let target = [
            0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ];

        // Hash with two leading zero bytes - should pass
        let good_hash = [
            0x00, 0x00, 0x12, 0x34, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ];
        assert!(hash_below_target(&good_hash, &target));

        // Hash with only one leading zero byte - should fail
        let bad_hash = [
            0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        assert!(!hash_below_target(&bad_hash, &target));
    }

    #[test]
    fn test_count_leading_zeros() {
        let hash1 = [0x00; 32]; // All zeros
        assert_eq!(count_leading_zeros(&hash1), 256);

        // Leading zeros in DISPLAY format = trailing zeros in internal format
        let mut hash2 = [0xFF; 32];
        hash2[31] = 0x00; // Last byte (first in display)
        hash2[30] = 0x00; // Second-to-last byte (second in display)
        hash2[29] = 0x0F; // Third-to-last byte - 4 leading zeros in this byte
        assert_eq!(count_leading_zeros(&hash2), 20); // 8 + 8 + 4
    }

    #[test]
    fn test_reverse_bytes() {
        let original = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10,
            0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
            0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F, 0x20,
        ];
        let reversed = reverse_bytes(&original);

        assert_eq!(reversed[0], 0x20);
        assert_eq!(reversed[31], 0x01);
    }
}
