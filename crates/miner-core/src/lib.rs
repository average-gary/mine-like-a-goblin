//! Core Bitcoin mining logic for the scratch-off miner application.
//!
//! This crate provides pure Rust implementations of:
//! - Bitcoin address validation (P2PKH, P2SH, P2WPKH, P2WSH, P2TR)
//! - Block header construction and serialization
//! - Coinbase transaction building with BIP34 compliance
//! - SHA256 double-hashing for mining
//! - Difficulty target conversion and comparison

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod address;
pub mod block;
pub mod coinbase;
pub mod difficulty;
pub mod hash;
pub mod merkle;
pub mod network;

pub use address::{validate_address, AddressError, AddressType, ValidatedAddress};
pub use block::BlockInfo;
pub use block::{BlockHeader, BlockTemplate};
pub use coinbase::CoinbaseBuilder;
pub use difficulty::{bits_to_target, hash_meets_target};
pub use hash::{double_sha256, mine_batch, MiningResult};
pub use merkle::compute_merkle_root;
pub use network::Network;
