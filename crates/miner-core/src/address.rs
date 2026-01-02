//! Bitcoin address validation and scriptPubKey generation.
//!
//! Supports:
//! - P2PKH (Pay to Public Key Hash) - Legacy addresses starting with 1 (mainnet) or m/n (testnet)
//! - P2SH (Pay to Script Hash) - Addresses starting with 3 (mainnet) or 2 (testnet)
//! - P2WPKH (Pay to Witness Public Key Hash) - Native SegWit v0, bc1q.../tb1q...
//! - P2WSH (Pay to Witness Script Hash) - Native SegWit v0, bc1q... (32-byte program)
//! - P2TR (Pay to Taproot) - SegWit v1, bc1p.../tb1p...

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use crate::hash::double_sha256;
use crate::network::Network;

/// Address validation errors.
#[derive(Debug, Clone)]
pub enum AddressError {
    /// Invalid address format
    InvalidFormat,
    /// Invalid Base58 character
    InvalidBase58Char(char),
    /// Invalid checksum
    InvalidChecksum,
    /// Invalid Bech32 encoding
    InvalidBech32(String),
    /// Invalid witness version
    InvalidWitnessVersion(u8),
    /// Invalid witness program length
    InvalidWitnessProgramLength(usize),
    /// Address network mismatch
    NetworkMismatch { expected: String, got: String },
    /// Unsupported address type
    UnsupportedType,
}

impl core::fmt::Display for AddressError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AddressError::InvalidFormat => write!(f, "Invalid address format"),
            AddressError::InvalidBase58Char(c) => write!(f, "Invalid Base58 character: {}", c),
            AddressError::InvalidChecksum => write!(f, "Invalid checksum"),
            AddressError::InvalidBech32(s) => write!(f, "Invalid Bech32 encoding: {}", s),
            AddressError::InvalidWitnessVersion(v) => write!(f, "Invalid witness version: {}", v),
            AddressError::InvalidWitnessProgramLength(l) => write!(f, "Invalid witness program length: {}", l),
            AddressError::NetworkMismatch { expected, got } => {
                write!(f, "Address network mismatch: expected {}, got {}", expected, got)
            }
            AddressError::UnsupportedType => write!(f, "Unsupported address type"),
        }
    }
}

/// Bitcoin address type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressType {
    /// Legacy P2PKH: OP_DUP OP_HASH160 <20-byte-hash> OP_EQUALVERIFY OP_CHECKSIG
    P2PKH,
    /// P2SH: OP_HASH160 <20-byte-hash> OP_EQUAL
    P2SH,
    /// Native SegWit v0 P2WPKH: OP_0 <20-byte-hash>
    P2WPKH,
    /// Native SegWit v0 P2WSH: OP_0 <32-byte-hash>
    P2WSH,
    /// Taproot P2TR: OP_1 <32-byte-x-only-pubkey>
    P2TR,
}

impl AddressType {
    /// Get the display name for this address type.
    pub fn name(&self) -> &'static str {
        match self {
            AddressType::P2PKH => "P2PKH",
            AddressType::P2SH => "P2SH",
            AddressType::P2WPKH => "P2WPKH",
            AddressType::P2WSH => "P2WSH",
            AddressType::P2TR => "P2TR",
        }
    }
}

/// A validated Bitcoin address with its scriptPubKey.
#[derive(Debug, Clone)]
pub struct ValidatedAddress {
    /// The type of address.
    pub address_type: AddressType,
    /// The network this address belongs to.
    pub network: Network,
    /// The scriptPubKey for this address (used in transaction outputs).
    pub script_pubkey: Vec<u8>,
    /// The original address string.
    pub display: String,
}

/// Validate a Bitcoin address and return its details.
pub fn validate_address(address: &str, expected_network: Network) -> Result<ValidatedAddress, AddressError> {
    let trimmed = address.trim();

    // Try Bech32/Bech32m first (bc1.../tb1...)
    if trimmed.to_lowercase().starts_with("bc1") || trimmed.to_lowercase().starts_with("tb1") {
        return validate_bech32_address(trimmed, expected_network);
    }

    // Try Base58Check (1.../3.../m.../n.../2...)
    validate_base58_address(trimmed, expected_network)
}

/// Validate a Base58Check encoded address (P2PKH or P2SH).
fn validate_base58_address(address: &str, expected_network: Network) -> Result<ValidatedAddress, AddressError> {
    // Decode Base58
    let decoded = base58_decode(address)?;

    if decoded.len() < 5 {
        return Err(AddressError::InvalidFormat);
    }

    // Verify checksum (last 4 bytes)
    let payload = &decoded[..decoded.len() - 4];
    let checksum = &decoded[decoded.len() - 4..];
    let computed_checksum = &double_sha256(payload)[..4];

    if checksum != computed_checksum {
        return Err(AddressError::InvalidChecksum);
    }

    // Extract version byte and hash
    let version = payload[0];
    let hash = &payload[1..];

    if hash.len() != 20 {
        return Err(AddressError::InvalidFormat);
    }

    // Determine address type and network
    let (address_type, network) = match version {
        0x00 => (AddressType::P2PKH, Network::Mainnet),
        0x05 => (AddressType::P2SH, Network::Mainnet),
        0x6f => (AddressType::P2PKH, Network::Testnet4),
        0xc4 => (AddressType::P2SH, Network::Testnet4),
        _ => return Err(AddressError::InvalidFormat),
    };

    // Check network matches
    if network != expected_network {
        return Err(AddressError::NetworkMismatch {
            expected: expected_network.name().into(),
            got: network.name().into(),
        });
    }

    // Build scriptPubKey
    let script_pubkey = match address_type {
        AddressType::P2PKH => {
            // OP_DUP OP_HASH160 <20-byte-hash> OP_EQUALVERIFY OP_CHECKSIG
            let mut script = Vec::with_capacity(25);
            script.push(0x76); // OP_DUP
            script.push(0xa9); // OP_HASH160
            script.push(0x14); // Push 20 bytes
            script.extend_from_slice(hash);
            script.push(0x88); // OP_EQUALVERIFY
            script.push(0xac); // OP_CHECKSIG
            script
        }
        AddressType::P2SH => {
            // OP_HASH160 <20-byte-hash> OP_EQUAL
            let mut script = Vec::with_capacity(23);
            script.push(0xa9); // OP_HASH160
            script.push(0x14); // Push 20 bytes
            script.extend_from_slice(hash);
            script.push(0x87); // OP_EQUAL
            script
        }
        _ => unreachable!(),
    };

    Ok(ValidatedAddress {
        address_type,
        network,
        script_pubkey,
        display: address.to_string(),
    })
}

/// Validate a Bech32/Bech32m encoded address (P2WPKH, P2WSH, or P2TR).
fn validate_bech32_address(address: &str, expected_network: Network) -> Result<ValidatedAddress, AddressError> {
    // Decode Bech32
    let (hrp, data, variant) = bech32_decode(address)?;

    // Check HRP matches network
    let network = match hrp.as_str() {
        "bc" => Network::Mainnet,
        "tb" => Network::Testnet4,
        _ => return Err(AddressError::InvalidBech32(format!("Unknown HRP: {}", hrp))),
    };

    if network != expected_network {
        return Err(AddressError::NetworkMismatch {
            expected: expected_network.name().into(),
            got: network.name().into(),
        });
    }

    if data.is_empty() {
        return Err(AddressError::InvalidFormat);
    }

    // First byte is witness version
    let witness_version = data[0];

    // Convert remaining 5-bit data to 8-bit
    let program = convert_bits(&data[1..], 5, 8, false)?;

    // Validate witness version and variant
    match witness_version {
        0 => {
            // SegWit v0 must use Bech32 (not Bech32m)
            if variant != Bech32Variant::Bech32 {
                return Err(AddressError::InvalidBech32("SegWit v0 must use Bech32".into()));
            }
        }
        1..=16 => {
            // SegWit v1+ must use Bech32m
            if variant != Bech32Variant::Bech32m {
                return Err(AddressError::InvalidBech32("SegWit v1+ must use Bech32m".into()));
            }
        }
        _ => return Err(AddressError::InvalidWitnessVersion(witness_version)),
    }

    // Determine address type based on version and program length
    let address_type = match (witness_version, program.len()) {
        (0, 20) => AddressType::P2WPKH,
        (0, 32) => AddressType::P2WSH,
        (1, 32) => AddressType::P2TR,
        (v, len) if v > 1 && (2..=40).contains(&len) => {
            // Future witness versions - we'll treat as unsupported for now
            return Err(AddressError::UnsupportedType);
        }
        (_, len) => return Err(AddressError::InvalidWitnessProgramLength(len)),
    };

    // Build scriptPubKey: OP_n <program>
    // OP_0 = 0x00, OP_1 = 0x51, OP_2 = 0x52, etc.
    let version_opcode = if witness_version == 0 { 0x00 } else { 0x50 + witness_version };
    let mut script_pubkey = Vec::with_capacity(2 + program.len());
    script_pubkey.push(version_opcode);
    script_pubkey.push(program.len() as u8);
    script_pubkey.extend_from_slice(&program);

    Ok(ValidatedAddress {
        address_type,
        network,
        script_pubkey,
        display: address.to_string(),
    })
}

// ============================================================================
// Base58 Implementation
// ============================================================================

const BASE58_ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

fn base58_decode(input: &str) -> Result<Vec<u8>, AddressError> {
    let mut result = Vec::new();

    // Count leading '1's (they become leading zeros)
    let mut leading_zeros = 0;
    for c in input.chars() {
        if c == '1' {
            leading_zeros += 1;
        } else {
            break;
        }
    }

    // Process remaining characters
    for c in input.chars() {
        let value = BASE58_ALPHABET
            .iter()
            .position(|&x| x == c as u8)
            .ok_or(AddressError::InvalidBase58Char(c))? as u32;

        // Multiply result by 58 and add value
        let mut carry = value;
        for byte in result.iter_mut().rev() {
            let temp = (*byte as u32) * 58 + carry;
            *byte = (temp & 0xFF) as u8;
            carry = temp >> 8;
        }

        while carry > 0 {
            result.insert(0, (carry & 0xFF) as u8);
            carry >>= 8;
        }
    }

    // Add leading zeros
    let mut final_result = vec![0u8; leading_zeros];
    final_result.extend(result);

    Ok(final_result)
}

// ============================================================================
// Bech32/Bech32m Implementation
// ============================================================================

const BECH32_CHARSET: &str = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bech32Variant {
    Bech32,
    Bech32m,
}

fn bech32_decode(input: &str) -> Result<(String, Vec<u8>, Bech32Variant), AddressError> {
    let input_lower = input.to_lowercase();

    // Find separator
    let sep_pos = input_lower.rfind('1')
        .ok_or(AddressError::InvalidBech32("No separator found".into()))?;

    if sep_pos == 0 || sep_pos + 7 > input_lower.len() {
        return Err(AddressError::InvalidBech32("Invalid separator position".into()));
    }

    let hrp = &input_lower[..sep_pos];
    let data_part = &input_lower[sep_pos + 1..];

    // Decode data characters
    let mut data = Vec::with_capacity(data_part.len());
    for c in data_part.chars() {
        let idx = BECH32_CHARSET
            .find(c)
            .ok_or(AddressError::InvalidBech32(format!("Invalid character: {}", c)))?;
        data.push(idx as u8);
    }

    // Verify checksum and determine variant
    let checksum = bech32_polymod(&hrp_expand(hrp), &data);

    let variant = if checksum == 1 {
        Bech32Variant::Bech32
    } else if checksum == 0x2bc830a3 {
        Bech32Variant::Bech32m
    } else {
        return Err(AddressError::InvalidBech32("Invalid checksum".into()));
    };

    // Remove checksum from data (last 6 characters)
    data.truncate(data.len() - 6);

    Ok((hrp.to_string(), data, variant))
}

fn hrp_expand(hrp: &str) -> Vec<u8> {
    let mut result = Vec::with_capacity(hrp.len() * 2 + 1);

    for c in hrp.chars() {
        result.push((c as u8) >> 5);
    }
    result.push(0);
    for c in hrp.chars() {
        result.push((c as u8) & 31);
    }

    result
}

fn bech32_polymod(hrp: &[u8], data: &[u8]) -> u32 {
    const GEN: [u32; 5] = [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3];

    let mut chk: u32 = 1;

    for &value in hrp.iter().chain(data.iter()) {
        let top = chk >> 25;
        chk = ((chk & 0x1ffffff) << 5) ^ (value as u32);
        for (i, &g) in GEN.iter().enumerate() {
            if (top >> i) & 1 == 1 {
                chk ^= g;
            }
        }
    }

    chk
}

fn convert_bits(data: &[u8], from_bits: u8, to_bits: u8, pad: bool) -> Result<Vec<u8>, AddressError> {
    let mut acc: u32 = 0;
    let mut bits: u8 = 0;
    let mut result = Vec::new();
    let max_value = (1u32 << to_bits) - 1;

    for &value in data {
        if value >> from_bits != 0 {
            return Err(AddressError::InvalidBech32("Invalid value in data".into()));
        }
        acc = (acc << from_bits) | (value as u32);
        bits += from_bits;

        while bits >= to_bits {
            bits -= to_bits;
            result.push(((acc >> bits) & max_value) as u8);
        }
    }

    if pad {
        if bits > 0 {
            result.push(((acc << (to_bits - bits)) & max_value) as u8);
        }
    } else if bits >= from_bits || ((acc << (to_bits - bits)) & max_value) != 0 {
        return Err(AddressError::InvalidBech32("Invalid padding".into()));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_p2pkh_mainnet() {
        // A known mainnet P2PKH address
        let address = "1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2";
        let result = validate_address(address, Network::Mainnet).unwrap();

        assert_eq!(result.address_type, AddressType::P2PKH);
        assert_eq!(result.network, Network::Mainnet);
        assert_eq!(result.script_pubkey.len(), 25);
        assert_eq!(result.script_pubkey[0], 0x76); // OP_DUP
        assert_eq!(result.script_pubkey[1], 0xa9); // OP_HASH160
    }

    #[test]
    fn test_p2sh_mainnet() {
        // A known mainnet P2SH address
        let address = "3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy";
        let result = validate_address(address, Network::Mainnet).unwrap();

        assert_eq!(result.address_type, AddressType::P2SH);
        assert_eq!(result.network, Network::Mainnet);
        assert_eq!(result.script_pubkey.len(), 23);
        assert_eq!(result.script_pubkey[0], 0xa9); // OP_HASH160
    }

    #[test]
    fn test_p2wpkh_mainnet() {
        // A known mainnet P2WPKH address
        let address = "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq";
        let result = validate_address(address, Network::Mainnet).unwrap();

        assert_eq!(result.address_type, AddressType::P2WPKH);
        assert_eq!(result.network, Network::Mainnet);
        assert_eq!(result.script_pubkey.len(), 22);
        assert_eq!(result.script_pubkey[0], 0x00); // OP_0
        assert_eq!(result.script_pubkey[1], 0x14); // Push 20 bytes
    }

    #[test]
    fn test_p2tr_mainnet() {
        // A known mainnet P2TR (Taproot) address
        let address = "bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr";
        let result = validate_address(address, Network::Mainnet).unwrap();

        assert_eq!(result.address_type, AddressType::P2TR);
        assert_eq!(result.network, Network::Mainnet);
        assert_eq!(result.script_pubkey.len(), 34);
        assert_eq!(result.script_pubkey[0], 0x51); // OP_1
        assert_eq!(result.script_pubkey[1], 0x20); // Push 32 bytes
    }

    #[test]
    fn test_testnet_address() {
        // A testnet P2WPKH address
        let address = "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx";
        let result = validate_address(address, Network::Testnet4).unwrap();

        assert_eq!(result.address_type, AddressType::P2WPKH);
        assert_eq!(result.network, Network::Testnet4);
    }

    #[test]
    fn test_network_mismatch() {
        // Try to validate a mainnet address with testnet expected
        let address = "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq";
        let result = validate_address(address, Network::Testnet4);

        assert!(matches!(result, Err(AddressError::NetworkMismatch { .. })));
    }

    #[test]
    fn test_invalid_checksum() {
        // Modified address with bad checksum
        let address = "1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN3"; // Changed last char
        let result = validate_address(address, Network::Mainnet);

        assert!(matches!(result, Err(AddressError::InvalidChecksum)));
    }
}
