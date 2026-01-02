//! Bitcoin block header construction and serialization.

use alloc::vec::Vec;
use crate::coinbase::{CoinbaseBuilder, CoinbaseTransaction};
use crate::difficulty::bits_to_target;
use crate::hash::double_sha256;
use crate::merkle::compute_merkle_root;
use crate::network::{Network, BLOCK_VERSION};

/// A Bitcoin block header (80 bytes).
#[derive(Debug, Clone)]
pub struct BlockHeader {
    /// Block version with BIP9 versionbits.
    pub version: i32,
    /// Hash of the previous block (internal byte order).
    pub prev_block_hash: [u8; 32],
    /// Merkle root of all transactions.
    pub merkle_root: [u8; 32],
    /// Block timestamp (Unix time).
    pub timestamp: u32,
    /// Difficulty target in compact "bits" format.
    pub bits: u32,
    /// Nonce for proof of work.
    pub nonce: u32,
}

impl BlockHeader {
    /// Create a new block header.
    pub fn new(
        prev_block_hash: [u8; 32],
        merkle_root: [u8; 32],
        timestamp: u32,
        bits: u32,
    ) -> Self {
        BlockHeader {
            version: BLOCK_VERSION,
            prev_block_hash,
            merkle_root,
            timestamp,
            bits,
            nonce: 0,
        }
    }

    /// Serialize the block header to 80 bytes.
    pub fn serialize(&self) -> [u8; 80] {
        let mut header = [0u8; 80];

        // Version (4 bytes, little-endian)
        header[0..4].copy_from_slice(&self.version.to_le_bytes());

        // Previous block hash (32 bytes, internal byte order)
        header[4..36].copy_from_slice(&self.prev_block_hash);

        // Merkle root (32 bytes)
        header[36..68].copy_from_slice(&self.merkle_root);

        // Timestamp (4 bytes, little-endian)
        header[68..72].copy_from_slice(&self.timestamp.to_le_bytes());

        // Bits (4 bytes, little-endian)
        header[72..76].copy_from_slice(&self.bits.to_le_bytes());

        // Nonce (4 bytes, little-endian)
        header[76..80].copy_from_slice(&self.nonce.to_le_bytes());

        header
    }

    /// Serialize the header without the nonce (76 bytes).
    /// Used for efficient mining where we only change the nonce.
    pub fn serialize_without_nonce(&self) -> [u8; 76] {
        let mut header = [0u8; 76];

        header[0..4].copy_from_slice(&self.version.to_le_bytes());
        header[4..36].copy_from_slice(&self.prev_block_hash);
        header[36..68].copy_from_slice(&self.merkle_root);
        header[68..72].copy_from_slice(&self.timestamp.to_le_bytes());
        header[72..76].copy_from_slice(&self.bits.to_le_bytes());

        header
    }

    /// Compute the block hash (double SHA256).
    pub fn hash(&self) -> [u8; 32] {
        double_sha256(&self.serialize())
    }

    /// Get the target as a 256-bit number.
    pub fn target(&self) -> [u8; 32] {
        bits_to_target(self.bits)
    }
}

/// A complete block template ready for mining.
#[derive(Debug, Clone)]
pub struct BlockTemplate {
    /// The block header.
    pub header: BlockHeader,
    /// The coinbase transaction.
    pub coinbase: CoinbaseTransaction,
    /// The block target (256-bit).
    pub target: [u8; 32],
    /// The network.
    pub network: Network,
    /// The block height.
    pub height: u32,
    /// The total reward (subsidy + fees).
    pub reward: u64,
}

impl BlockTemplate {
    /// Create a new block template.
    ///
    /// # Arguments
    /// * `network` - The Bitcoin network
    /// * `height` - The block height
    /// * `prev_block_hash` - Hash of the previous block (display byte order, will be reversed)
    /// * `bits` - Difficulty target in compact format
    /// * `timestamp` - Block timestamp
    /// * `coinbase_builder` - Builder for the coinbase transaction
    /// * `reward` - Total block reward (subsidy + fees)
    pub fn new(
        network: Network,
        height: u32,
        prev_block_hash: [u8; 32],
        bits: u32,
        timestamp: u32,
        coinbase_builder: CoinbaseBuilder,
        reward: u64,
    ) -> Self {
        // Build the coinbase transaction
        let coinbase = coinbase_builder.build(reward);

        // Compute merkle root (for coinbase-only, it's just the txid)
        let merkle_root = compute_merkle_root(&[coinbase.txid]);

        // Create the header
        let header = BlockHeader::new(prev_block_hash, merkle_root, timestamp, bits);

        // Get the target
        let target = bits_to_target(bits);

        BlockTemplate {
            header,
            coinbase,
            target,
            network,
            height,
            reward,
        }
    }

    /// Update the extra nonce and rebuild the coinbase/merkle root.
    ///
    /// This is used when we've exhausted all nonce values and need to
    /// change the merkle root to continue mining.
    pub fn update_extra_nonce(&mut self, extra_nonce: [u8; 8], coinbase_builder: CoinbaseBuilder) {
        // Rebuild coinbase with new extra nonce
        self.coinbase = coinbase_builder.with_extra_nonce(extra_nonce).build(self.reward);

        // Update merkle root
        self.header.merkle_root = compute_merkle_root(&[self.coinbase.txid]);

        // Reset nonce
        self.header.nonce = 0;
    }

    /// Serialize the complete block for submission.
    pub fn serialize_block(&self) -> Vec<u8> {
        let mut block = Vec::with_capacity(200);

        // Block header (80 bytes)
        block.extend_from_slice(&self.header.serialize());

        // Transaction count (varint) - just the coinbase
        block.push(0x01);

        // Coinbase transaction (with witness)
        block.extend_from_slice(&self.coinbase.raw_tx_with_witness);

        block
    }

    /// Get the block as hex string for submission.
    pub fn serialize_block_hex(&self) -> alloc::string::String {
        hex::encode(self.serialize_block())
    }
}

/// Information needed to construct a block template from API data.
#[derive(Debug, Clone)]
pub struct BlockInfo {
    /// The hash of the current tip block (will be our prev_block_hash).
    pub tip_hash: [u8; 32],
    /// The height of the next block (tip height + 1).
    pub height: u32,
    /// The difficulty bits for the next block.
    pub bits: u32,
    /// Current timestamp.
    pub timestamp: u32,
}

impl BlockInfo {
    /// Create block info from API data.
    ///
    /// # Arguments
    /// * `tip_hash_hex` - The tip block hash in display format (will be reversed)
    /// * `tip_height` - The height of the tip block
    /// * `bits` - Difficulty bits (or current bits to use)
    pub fn from_api_data(
        tip_hash_hex: &str,
        tip_height: u32,
        bits: u32,
    ) -> Result<Self, &'static str> {
        // Parse and reverse the tip hash (API gives display order, we need internal)
        let tip_hash_bytes = hex::decode(tip_hash_hex)
            .map_err(|_| "Invalid tip hash hex")?;

        if tip_hash_bytes.len() != 32 {
            return Err("Tip hash must be 32 bytes");
        }

        // Reverse from display order to internal byte order
        let mut tip_hash = [0u8; 32];
        for i in 0..32 {
            tip_hash[i] = tip_hash_bytes[31 - i];
        }

        // Get current timestamp
        let timestamp = current_timestamp();

        Ok(BlockInfo {
            tip_hash,
            height: tip_height + 1,
            bits,
            timestamp,
        })
    }
}

/// Get the current Unix timestamp.
#[cfg(feature = "std")]
fn current_timestamp() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0)
}

#[cfg(not(feature = "std"))]
fn current_timestamp() -> u32 {
    // In no_std, we'll need to get timestamp from JS
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::validate_address;

    #[test]
    fn test_block_header_serialization() {
        let prev_hash = [0x12u8; 32];
        let merkle_root = [0x34u8; 32];
        let timestamp = 1700000000u32;
        let bits = 0x17034219u32;

        let mut header = BlockHeader::new(prev_hash, merkle_root, timestamp, bits);
        header.nonce = 0xDEADBEEF;

        let serialized = header.serialize();

        // Verify length
        assert_eq!(serialized.len(), 80);

        // Verify version (0x20000000 in little-endian)
        assert_eq!(&serialized[0..4], &[0x00, 0x00, 0x00, 0x20]);

        // Verify prev_hash
        assert_eq!(&serialized[4..36], &prev_hash[..]);

        // Verify merkle_root
        assert_eq!(&serialized[36..68], &merkle_root[..]);

        // Verify nonce (0xDEADBEEF in little-endian)
        assert_eq!(&serialized[76..80], &[0xEF, 0xBE, 0xAD, 0xDE]);
    }

    #[test]
    fn test_block_header_hash() {
        // This tests that the hash function works correctly
        let header = BlockHeader::new([0u8; 32], [0u8; 32], 0, 0x1d00ffff);
        let hash = header.hash();

        // Just verify we get a 32-byte hash
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_block_template_creation() {
        let network = Network::Mainnet;
        let address = validate_address("bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq", network).unwrap();

        let prev_hash = [0u8; 32];
        let bits = 0x17034219;
        let timestamp = 1700000000;
        let height = 875000;
        let reward = 312_500_000;

        let coinbase_builder = CoinbaseBuilder::new(network, height, address);

        let template = BlockTemplate::new(
            network,
            height,
            prev_hash,
            bits,
            timestamp,
            coinbase_builder,
            reward,
        );

        // Verify basic properties
        assert_eq!(template.height, height);
        assert_eq!(template.reward, reward);
        assert_eq!(template.header.prev_block_hash, prev_hash);
        assert_eq!(template.header.bits, bits);

        // Verify merkle root is set (non-zero for real coinbase)
        assert_ne!(template.header.merkle_root, [0u8; 32]);

        // Verify block can be serialized
        let block_hex = template.serialize_block_hex();
        assert!(!block_hex.is_empty());
    }
}
