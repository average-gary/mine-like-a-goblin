//! Mining controller for the WASM miner.

use wasm_bindgen::prelude::*;
use miner_core::{
    validate_address, BlockTemplate, CoinbaseBuilder, Network, mine_batch,
    hash::{count_leading_zeros, hash_to_display_hex},
    network::SHARE_MIN_LEADING_ZEROS,
    difficulty::{bits_to_difficulty, format_difficulty},
};
use crate::state::{MiningStats, TemplateInfo, MiningResultInfo};

/// The main mining controller.
#[wasm_bindgen]
pub struct Miner {
    /// The network being mined.
    network: Network,
    /// The validated reward address.
    address: miner_core::ValidatedAddress,
    /// The current block template.
    template: Option<BlockTemplate>,
    /// Mining statistics.
    stats: MiningStats,
    /// Start time of mining.
    start_time: f64,
    /// Whether mining is active.
    is_mining: bool,
    /// Current nonce position.
    current_nonce: u32,
    /// Extra nonce for merkle root variation.
    extra_nonce: u64,
    /// Best hash found so far.
    best_hash: Option<[u8; 32]>,
}

#[wasm_bindgen]
impl Miner {
    /// Create a new miner instance.
    ///
    /// # Arguments
    /// * `address` - The Bitcoin address to receive mining rewards
    /// * `network` - The network ("mainnet" or "testnet4")
    #[wasm_bindgen(constructor)]
    pub fn new(address: &str, network: &str) -> Result<Miner, JsValue> {
        let net = Network::from_str(network)
            .ok_or_else(|| JsValue::from_str("Invalid network"))?;

        let validated_address = validate_address(address, net)
            .map_err(|e| JsValue::from_str(&format!("Invalid address: {}", e)))?;

        Ok(Miner {
            network: net,
            address: validated_address,
            template: None,
            stats: MiningStats::new(),
            start_time: 0.0,
            is_mining: false,
            current_nonce: 0,
            extra_nonce: 0,
            best_hash: None,
        })
    }

    /// Validate a Bitcoin address for the current network.
    #[wasm_bindgen]
    pub fn validate_address(address: &str, network: &str) -> Result<bool, JsValue> {
        let net = Network::from_str(network)
            .ok_or_else(|| JsValue::from_str("Invalid network"))?;

        match validate_address(address, net) {
            Ok(_) => Ok(true),
            Err(e) => Err(JsValue::from_str(&format!("{}", e))),
        }
    }

    /// Build a block template from API data.
    ///
    /// # Arguments
    /// * `tip_hash` - The current tip block hash
    /// * `tip_height` - The current tip block height
    /// * `bits` - The difficulty bits
    /// * `timestamp` - The block timestamp (or 0 to use current time)
    #[wasm_bindgen]
    pub fn build_template(
        &mut self,
        tip_hash: &str,
        tip_height: u32,
        bits: u32,
        timestamp: u32,
    ) -> Result<JsValue, JsValue> {
        // Parse and reverse the tip hash (API gives display order)
        let tip_hash_bytes = hex::decode(tip_hash)
            .map_err(|_| JsValue::from_str("Invalid tip hash hex"))?;

        if tip_hash_bytes.len() != 32 {
            return Err(JsValue::from_str("Tip hash must be 32 bytes"));
        }

        // Reverse from display order to internal byte order
        let mut prev_hash = [0u8; 32];
        for i in 0..32 {
            prev_hash[i] = tip_hash_bytes[31 - i];
        }

        // Get timestamp (use provided or current time)
        let ts = if timestamp > 0 {
            timestamp
        } else {
            (js_sys::Date::now() / 1000.0) as u32
        };

        // Calculate block reward
        let height = tip_height + 1;
        let reward = self.network.block_subsidy(height);

        // Build coinbase
        let coinbase_builder = CoinbaseBuilder::new(
            self.network,
            height,
            self.address.clone(),
        ).with_extra_nonce(self.extra_nonce.to_le_bytes());

        // Create template
        let template = BlockTemplate::new(
            self.network,
            height,
            prev_hash,
            bits,
            ts,
            coinbase_builder,
            reward,
        );

        // Calculate difficulty
        let difficulty = bits_to_difficulty(bits);

        // Create template info
        let info = TemplateInfo {
            height,
            prev_hash: tip_hash.to_string(),
            bits,
            difficulty,
            difficulty_display: format_difficulty(difficulty),
            reward,
            reward_btc: reward as f64 / 100_000_000.0,
            network: self.network.name().to_string(),
            address: self.address.display.clone(),
        };

        self.template = Some(template);
        self.current_nonce = 0;
        self.stats = MiningStats::new();
        self.best_hash = None;

        info.to_js()
    }

    /// Mine a batch of nonces.
    ///
    /// # Arguments
    /// * `batch_size` - Number of nonces to try in this batch
    ///
    /// # Returns
    /// Mining result with share/block found status and statistics.
    #[wasm_bindgen]
    pub fn mine_batch(&mut self, batch_size: u32) -> Result<JsValue, JsValue> {
        let template = self.template.as_mut()
            .ok_or_else(|| JsValue::from_str("No template built"))?;

        // Get header without nonce for efficient hashing
        template.header.nonce = self.current_nonce;
        let header_without_nonce = template.header.serialize_without_nonce();

        // Mine the batch
        let result = mine_batch(
            &header_without_nonce,
            &template.target,
            SHARE_MIN_LEADING_ZEROS,
            self.current_nonce,
            batch_size,
        );

        // Update statistics
        self.stats.total_hashes += result.hashes_computed;
        self.current_nonce = self.current_nonce.saturating_add(batch_size);
        self.stats.current_nonce = self.current_nonce;

        // Update elapsed time
        if self.start_time > 0.0 {
            let now = js_sys::Date::now();
            self.stats.elapsed_ms = now - self.start_time;
            self.stats.update_hash_rate();
        }

        // Create result info
        let mut info = MiningResultInfo {
            share_found: result.share_found,
            block_found: result.block_found,
            nonce: result.nonce,
            hash: None,
            leading_zeros: 0,
            hashes_computed: result.hashes_computed,
        };

        // Handle found results
        if let (Some(nonce), Some(hash)) = (result.nonce, result.hash) {
            let leading_zeros = count_leading_zeros(&hash);
            info.leading_zeros = leading_zeros;
            info.hash = Some(hash_to_display_hex(&hash));

            // Update best hash if this is better
            let is_better = match &self.best_hash {
                None => true,
                Some(best) => hash < *best,
            };

            if is_better {
                self.best_hash = Some(hash);
                self.stats.best_hash = Some(hash_to_display_hex(&hash));
                self.stats.best_leading_zeros = leading_zeros;
            }

            if result.share_found {
                self.stats.shares_found += 1;
            }

            if result.block_found {
                self.stats.block_found = true;
                // Set the winning nonce in the template
                template.header.nonce = nonce;
            }
        }

        // Check if we need to update extra nonce (nonce overflow)
        if self.current_nonce == u32::MAX {
            self.extra_nonce += 1;
            self.current_nonce = 0;
            // Would need to rebuild template here with new extra_nonce
        }

        info.to_js()
    }

    /// Start mining.
    #[wasm_bindgen]
    pub fn start_mining(&mut self) {
        self.is_mining = true;
        self.start_time = js_sys::Date::now();
    }

    /// Stop mining.
    #[wasm_bindgen]
    pub fn stop_mining(&mut self) {
        self.is_mining = false;
    }

    /// Check if mining is active.
    #[wasm_bindgen(getter)]
    pub fn is_mining(&self) -> bool {
        self.is_mining
    }

    /// Get current mining statistics.
    #[wasm_bindgen]
    pub fn get_stats(&self) -> Result<JsValue, JsValue> {
        self.stats.to_js()
    }

    /// Get the formatted hash rate.
    #[wasm_bindgen]
    pub fn get_hash_rate_display(&self) -> String {
        self.stats.format_hash_rate()
    }

    /// Get the serialized block for submission (if a valid block was found).
    #[wasm_bindgen]
    pub fn get_block_hex(&self) -> Option<String> {
        if self.stats.block_found {
            self.template.as_ref().map(|t| t.serialize_block_hex())
        } else {
            None
        }
    }

    /// Reset the miner for a new block.
    #[wasm_bindgen]
    pub fn reset(&mut self) {
        self.template = None;
        self.stats = MiningStats::new();
        self.current_nonce = 0;
        self.start_time = 0.0;
        self.is_mining = false;
        self.best_hash = None;
    }

    /// Get the current network.
    #[wasm_bindgen(getter)]
    pub fn network(&self) -> String {
        self.network.name().to_string()
    }

    /// Get the reward address.
    #[wasm_bindgen(getter)]
    pub fn address(&self) -> String {
        self.address.display.clone()
    }
}

/// Log to the browser console.
#[wasm_bindgen]
pub fn console_log(message: &str) {
    web_sys::console::log_1(&JsValue::from_str(message));
}
