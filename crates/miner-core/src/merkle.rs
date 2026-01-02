//! Merkle tree computation for Bitcoin transactions.

use alloc::vec::Vec;
use crate::hash::double_sha256;

/// Compute the merkle root from a list of transaction IDs.
///
/// For a single transaction (like our coinbase-only block), the merkle root
/// is simply the txid itself.
///
/// For multiple transactions, we build a binary tree of hashes.
pub fn compute_merkle_root(txids: &[[u8; 32]]) -> [u8; 32] {
    if txids.is_empty() {
        return [0u8; 32];
    }

    if txids.len() == 1 {
        return txids[0];
    }

    let mut current_level: Vec<[u8; 32]> = txids.to_vec();

    while current_level.len() > 1 {
        let mut next_level = Vec::with_capacity((current_level.len() + 1) / 2);

        for i in (0..current_level.len()).step_by(2) {
            let left = current_level[i];
            // If odd number of elements, duplicate the last one
            let right = if i + 1 < current_level.len() {
                current_level[i + 1]
            } else {
                current_level[i]
            };

            // Concatenate and hash
            let mut combined = [0u8; 64];
            combined[..32].copy_from_slice(&left);
            combined[32..].copy_from_slice(&right);
            next_level.push(double_sha256(&combined));
        }

        current_level = next_level;
    }

    current_level[0]
}

/// Compute the witness commitment for a SegWit block.
///
/// The witness commitment is: SHA256d(witness_merkle_root || witness_reserved_value)
///
/// For our coinbase-only block:
/// - The witness merkle root for a single coinbase is the coinbase's wtxid
/// - But the coinbase wtxid is defined as all zeros (32 zero bytes)
/// - So witness_merkle_root = 0x00...00 for coinbase-only blocks
///
/// # Arguments
/// * `witness_reserved_value` - The 32-byte witness reserved value from coinbase witness
pub fn compute_witness_commitment(witness_reserved_value: &[u8; 32]) -> [u8; 32] {
    // For coinbase-only block, witness merkle root is all zeros
    // because wtxid of coinbase is defined as all zeros
    let witness_merkle_root = [0u8; 32];

    let mut data = [0u8; 64];
    data[..32].copy_from_slice(&witness_merkle_root);
    data[32..].copy_from_slice(witness_reserved_value);

    double_sha256(&data)
}

/// Generate the scriptPubKey for a witness commitment output.
///
/// Format: OP_RETURN <commitment>
/// Where commitment = 0xaa21a9ed || witness_commitment
pub fn witness_commitment_script(witness_commitment: &[u8; 32]) -> Vec<u8> {
    let mut script = Vec::with_capacity(38);

    // OP_RETURN
    script.push(0x6a);

    // Push 36 bytes
    script.push(0x24);

    // Witness commitment header (magic bytes)
    script.extend_from_slice(&[0xaa, 0x21, 0xa9, 0xed]);

    // Witness commitment hash
    script.extend_from_slice(witness_commitment);

    script
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_tx_merkle_root() {
        let txid = [0x42u8; 32];
        let root = compute_merkle_root(&[txid]);
        assert_eq!(root, txid);
    }

    #[test]
    fn test_two_tx_merkle_root() {
        let tx1 = [0x11u8; 32];
        let tx2 = [0x22u8; 32];

        let root = compute_merkle_root(&[tx1, tx2]);

        // Manually compute expected root
        let mut combined = [0u8; 64];
        combined[..32].copy_from_slice(&tx1);
        combined[32..].copy_from_slice(&tx2);
        let expected = double_sha256(&combined);

        assert_eq!(root, expected);
    }

    #[test]
    fn test_three_tx_merkle_root() {
        // With 3 transactions, the third is duplicated
        let tx1 = [0x11u8; 32];
        let tx2 = [0x22u8; 32];
        let tx3 = [0x33u8; 32];

        let root = compute_merkle_root(&[tx1, tx2, tx3]);

        // Level 1: hash(tx1, tx2), hash(tx3, tx3)
        let mut combined12 = [0u8; 64];
        combined12[..32].copy_from_slice(&tx1);
        combined12[32..].copy_from_slice(&tx2);
        let h12 = double_sha256(&combined12);

        let mut combined33 = [0u8; 64];
        combined33[..32].copy_from_slice(&tx3);
        combined33[32..].copy_from_slice(&tx3);
        let h33 = double_sha256(&combined33);

        // Level 0: hash(h12, h33)
        let mut final_combined = [0u8; 64];
        final_combined[..32].copy_from_slice(&h12);
        final_combined[32..].copy_from_slice(&h33);
        let expected = double_sha256(&final_combined);

        assert_eq!(root, expected);
    }

    #[test]
    fn test_witness_commitment_script() {
        let commitment = [0xAB; 32];
        let script = witness_commitment_script(&commitment);

        assert_eq!(script.len(), 38);
        assert_eq!(script[0], 0x6a); // OP_RETURN
        assert_eq!(script[1], 0x24); // Push 36 bytes
        assert_eq!(&script[2..6], &[0xaa, 0x21, 0xa9, 0xed]); // Magic
        assert_eq!(&script[6..], &commitment[..]); // Commitment
    }
}
