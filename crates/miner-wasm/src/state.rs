//! Application state management for the miner.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Mining statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MiningStats {
    /// Total hashes computed.
    pub total_hashes: u64,
    /// Current hash rate (hashes per second).
    pub hash_rate: f64,
    /// Number of shares found.
    pub shares_found: u32,
    /// Whether a valid block was found.
    pub block_found: bool,
    /// Current nonce value.
    pub current_nonce: u32,
    /// Elapsed time in milliseconds.
    pub elapsed_ms: f64,
    /// Best hash found (lowest).
    pub best_hash: Option<String>,
    /// Number of leading zeros in best hash.
    pub best_leading_zeros: u32,
}

impl MiningStats {
    /// Create new empty stats.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update hash rate based on elapsed time.
    pub fn update_hash_rate(&mut self) {
        if self.elapsed_ms > 0.0 {
            self.hash_rate = (self.total_hashes as f64) / (self.elapsed_ms / 1000.0);
        }
    }

    /// Format hash rate for display.
    pub fn format_hash_rate(&self) -> String {
        if self.hash_rate >= 1_000_000_000.0 {
            format!("{:.2} GH/s", self.hash_rate / 1_000_000_000.0)
        } else if self.hash_rate >= 1_000_000.0 {
            format!("{:.2} MH/s", self.hash_rate / 1_000_000.0)
        } else if self.hash_rate >= 1_000.0 {
            format!("{:.2} KH/s", self.hash_rate / 1_000.0)
        } else {
            format!("{:.2} H/s", self.hash_rate)
        }
    }

    /// Convert to JS value.
    pub fn to_js(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(self)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {:?}", e)))
    }
}

/// Block template information for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    /// Block height.
    pub height: u32,
    /// Previous block hash (display format).
    pub prev_hash: String,
    /// Difficulty bits.
    pub bits: u32,
    /// Difficulty as a number.
    pub difficulty: f64,
    /// Formatted difficulty string.
    pub difficulty_display: String,
    /// Block reward in satoshis.
    pub reward: u64,
    /// Block reward in BTC.
    pub reward_btc: f64,
    /// Network name.
    pub network: String,
    /// Miner's address.
    pub address: String,
}

impl TemplateInfo {
    /// Convert to JS value.
    pub fn to_js(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(self)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {:?}", e)))
    }
}

/// Result of a mining operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningResultInfo {
    /// Whether a share was found.
    pub share_found: bool,
    /// Whether a valid block was found.
    pub block_found: bool,
    /// The winning nonce (if found).
    pub nonce: Option<u32>,
    /// The block hash (if found).
    pub hash: Option<String>,
    /// Number of leading zeros in hash.
    pub leading_zeros: u32,
    /// Hashes computed in this batch.
    pub hashes_computed: u64,
}

impl MiningResultInfo {
    /// Convert to JS value.
    pub fn to_js(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(self)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {:?}", e)))
    }
}
