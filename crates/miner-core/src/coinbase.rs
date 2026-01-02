//! Coinbase transaction construction for Bitcoin mining.
//!
//! The coinbase transaction is the first transaction in a block that creates
//! new coins (the block reward) and collects transaction fees.

use alloc::vec;
use alloc::vec::Vec;
use crate::address::ValidatedAddress;
use crate::hash::double_sha256;
use crate::merkle::{compute_witness_commitment, witness_commitment_script};
use crate::network::Network;

/// Builder for constructing coinbase transactions.
pub struct CoinbaseBuilder {
    /// The network (mainnet or testnet4).
    #[allow(dead_code)]
    network: Network,
    /// The block height (required by BIP34).
    block_height: u32,
    /// The address to receive the block reward.
    reward_address: ValidatedAddress,
    /// Extra nonce data for merkle root variation (8 bytes).
    extra_nonce: [u8; 8],
    /// Witness reserved value (32 bytes, typically all zeros).
    witness_reserved: [u8; 32],
}

impl CoinbaseBuilder {
    /// Create a new coinbase builder.
    pub fn new(
        network: Network,
        block_height: u32,
        reward_address: ValidatedAddress,
    ) -> Self {
        CoinbaseBuilder {
            network,
            block_height,
            reward_address,
            extra_nonce: [0u8; 8],
            witness_reserved: [0u8; 32],
        }
    }

    /// Set the extra nonce (used to vary the merkle root).
    pub fn with_extra_nonce(mut self, extra_nonce: [u8; 8]) -> Self {
        self.extra_nonce = extra_nonce;
        self
    }

    /// Set the witness reserved value.
    pub fn with_witness_reserved(mut self, witness_reserved: [u8; 32]) -> Self {
        self.witness_reserved = witness_reserved;
        self
    }

    /// Build the coinbase transaction.
    ///
    /// Returns the serialized transaction and its txid.
    pub fn build(&self, total_reward: u64) -> CoinbaseTransaction {
        // Build scriptSig: [height_push] [height_bytes] [extra_nonce]
        let script_sig = self.build_script_sig();

        // Build outputs
        let outputs = self.build_outputs(total_reward);

        // Serialize the transaction
        let (raw_tx, raw_tx_with_witness) = self.serialize_transaction(&script_sig, &outputs);

        // Compute txid (hash of non-witness serialization)
        let txid = double_sha256(&raw_tx);

        // For coinbase, wtxid is defined as all zeros
        let wtxid = [0u8; 32];

        CoinbaseTransaction {
            raw_tx,
            raw_tx_with_witness,
            txid,
            wtxid,
        }
    }

    /// Build the scriptSig with BIP34 height encoding and extra nonce.
    fn build_script_sig(&self) -> Vec<u8> {
        let mut script_sig = Vec::with_capacity(32);

        // BIP34: Block height must be in scriptSig
        let height_bytes = encode_block_height(self.block_height);
        script_sig.push(height_bytes.len() as u8); // Push opcode
        script_sig.extend_from_slice(&height_bytes);

        // Add extra nonce
        script_sig.extend_from_slice(&self.extra_nonce);

        // Add miner tag (optional, for fun)
        let tag = b"/ScratchOffMiner/";
        script_sig.extend_from_slice(tag);

        script_sig
    }

    /// Build the transaction outputs.
    fn build_outputs(&self, total_reward: u64) -> Vec<TxOutput> {
        let mut outputs = Vec::with_capacity(2);

        // Output 0: Block reward to miner's address
        outputs.push(TxOutput {
            value: total_reward,
            script_pubkey: self.reward_address.script_pubkey.clone(),
        });

        // Output 1: Witness commitment (required for SegWit blocks)
        let witness_commitment = compute_witness_commitment(&self.witness_reserved);
        let commitment_script = witness_commitment_script(&witness_commitment);
        outputs.push(TxOutput {
            value: 0, // Witness commitment has no value
            script_pubkey: commitment_script,
        });

        outputs
    }

    /// Serialize the transaction (both with and without witness).
    fn serialize_transaction(
        &self,
        script_sig: &[u8],
        outputs: &[TxOutput],
    ) -> (Vec<u8>, Vec<u8>) {
        // Non-witness serialization (for txid)
        let mut raw_tx = Vec::with_capacity(200);

        // Version (4 bytes, little-endian)
        raw_tx.extend_from_slice(&2u32.to_le_bytes());

        // Input count (varint) - always 1 for coinbase
        raw_tx.push(0x01);

        // Input: Previous output (null for coinbase)
        raw_tx.extend_from_slice(&[0u8; 32]); // Previous txid (all zeros)
        raw_tx.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // Previous vout (all 1s)

        // ScriptSig length (varint)
        encode_varint(script_sig.len() as u64, &mut raw_tx);
        raw_tx.extend_from_slice(script_sig);

        // Sequence (4 bytes)
        raw_tx.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes());

        // Output count (varint)
        encode_varint(outputs.len() as u64, &mut raw_tx);

        // Outputs
        for output in outputs {
            raw_tx.extend_from_slice(&output.value.to_le_bytes());
            encode_varint(output.script_pubkey.len() as u64, &mut raw_tx);
            raw_tx.extend_from_slice(&output.script_pubkey);
        }

        // Locktime (4 bytes)
        raw_tx.extend_from_slice(&0u32.to_le_bytes());

        // Witness serialization (for network transmission)
        let mut raw_tx_with_witness = Vec::with_capacity(300);

        // Version
        raw_tx_with_witness.extend_from_slice(&2u32.to_le_bytes());

        // Marker and flag (SegWit indicator)
        raw_tx_with_witness.push(0x00); // Marker
        raw_tx_with_witness.push(0x01); // Flag

        // Input count
        raw_tx_with_witness.push(0x01);

        // Input
        raw_tx_with_witness.extend_from_slice(&[0u8; 32]); // Previous txid
        raw_tx_with_witness.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // Previous vout
        encode_varint(script_sig.len() as u64, &mut raw_tx_with_witness);
        raw_tx_with_witness.extend_from_slice(script_sig);
        raw_tx_with_witness.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // Sequence

        // Output count
        encode_varint(outputs.len() as u64, &mut raw_tx_with_witness);

        // Outputs
        for output in outputs {
            raw_tx_with_witness.extend_from_slice(&output.value.to_le_bytes());
            encode_varint(output.script_pubkey.len() as u64, &mut raw_tx_with_witness);
            raw_tx_with_witness.extend_from_slice(&output.script_pubkey);
        }

        // Witness data for coinbase
        // Coinbase must have exactly one witness stack with one 32-byte element
        raw_tx_with_witness.push(0x01); // Number of witness stack items
        raw_tx_with_witness.push(0x20); // Length of item (32 bytes)
        raw_tx_with_witness.extend_from_slice(&self.witness_reserved);

        // Locktime
        raw_tx_with_witness.extend_from_slice(&0u32.to_le_bytes());

        (raw_tx, raw_tx_with_witness)
    }
}

/// A constructed coinbase transaction.
#[derive(Debug, Clone)]
pub struct CoinbaseTransaction {
    /// Raw transaction without witness (used for txid calculation).
    pub raw_tx: Vec<u8>,
    /// Raw transaction with witness (used for network transmission).
    pub raw_tx_with_witness: Vec<u8>,
    /// Transaction ID (double SHA256 of raw_tx).
    pub txid: [u8; 32],
    /// Witness transaction ID (all zeros for coinbase).
    pub wtxid: [u8; 32],
}

/// A transaction output.
struct TxOutput {
    value: u64,
    script_pubkey: Vec<u8>,
}

/// Encode a block height according to BIP34.
///
/// The height is minimally encoded as a little-endian integer with proper handling
/// of the sign bit.
fn encode_block_height(height: u32) -> Vec<u8> {
    if height == 0 {
        // Special case: OP_0 for height 0
        return vec![];
    }

    // Convert to bytes (little-endian)
    let mut bytes = Vec::new();
    let mut n = height;

    while n > 0 {
        bytes.push((n & 0xFF) as u8);
        n >>= 8;
    }

    // If the high bit is set, append a 0x00 byte to prevent it being
    // interpreted as negative
    if let Some(&last) = bytes.last() {
        if last & 0x80 != 0 {
            bytes.push(0x00);
        }
    }

    bytes
}

/// Encode a variable-length integer (Bitcoin varint).
fn encode_varint(value: u64, output: &mut Vec<u8>) {
    if value < 0xfd {
        output.push(value as u8);
    } else if value <= 0xffff {
        output.push(0xfd);
        output.extend_from_slice(&(value as u16).to_le_bytes());
    } else if value <= 0xffffffff {
        output.push(0xfe);
        output.extend_from_slice(&(value as u32).to_le_bytes());
    } else {
        output.push(0xff);
        output.extend_from_slice(&value.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::{validate_address, AddressType};

    #[test]
    fn test_encode_block_height() {
        // Height 0
        assert_eq!(encode_block_height(0), vec![]);

        // Height 1
        assert_eq!(encode_block_height(1), vec![0x01]);

        // Height 127 (0x7F - max without sign bit issue)
        assert_eq!(encode_block_height(127), vec![0x7F]);

        // Height 128 (0x80 - needs padding to avoid negative)
        assert_eq!(encode_block_height(128), vec![0x80, 0x00]);

        // Height 256
        assert_eq!(encode_block_height(256), vec![0x00, 0x01]);

        // Height 500000 (example from mainnet)
        // 500000 = 0x07A120 in little-endian: 0x20, 0xA1, 0x07
        assert_eq!(encode_block_height(500000), vec![0x20, 0xA1, 0x07]);
    }

    #[test]
    fn test_encode_varint() {
        let mut output = Vec::new();

        // Small value (< 0xfd)
        encode_varint(100, &mut output);
        assert_eq!(output, vec![100]);

        // Medium value (0xfd - 0xffff)
        output.clear();
        encode_varint(0x1234, &mut output);
        assert_eq!(output, vec![0xfd, 0x34, 0x12]);
    }

    #[test]
    fn test_coinbase_builder() {
        let network = Network::Mainnet;
        let address = validate_address("bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq", network).unwrap();

        let builder = CoinbaseBuilder::new(network, 875000, address)
            .with_extra_nonce([0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);

        let reward = 312_500_000; // 3.125 BTC in satoshis
        let coinbase = builder.build(reward);

        // Verify txid is 32 bytes
        assert_eq!(coinbase.txid.len(), 32);

        // Verify wtxid is all zeros (for coinbase)
        assert_eq!(coinbase.wtxid, [0u8; 32]);

        // Verify raw_tx is non-empty
        assert!(!coinbase.raw_tx.is_empty());

        // Verify witness version is longer (has marker, flag, and witness)
        assert!(coinbase.raw_tx_with_witness.len() > coinbase.raw_tx.len());
    }
}
