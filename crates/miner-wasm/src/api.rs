//! Blockchain API integration for fetching block data.

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

/// Blockchain API client for fetching block data.
#[wasm_bindgen]
pub struct BlockchainApi {
    /// Base URL for the API
    base_url: String,
    /// Network name
    network: String,
}

#[wasm_bindgen]
impl BlockchainApi {
    /// Create a new API client for the specified network.
    #[wasm_bindgen(constructor)]
    pub fn new(network: &str) -> Self {
        let base_url = match network {
            "mainnet" | "main" => "https://mempool.space/api".to_string(),
            "testnet4" | "testnet" => "https://mempool.space/testnet4/api".to_string(),
            _ => "https://mempool.space/api".to_string(),
        };

        BlockchainApi {
            base_url,
            network: network.to_string(),
        }
    }

    /// Get the current tip block hash.
    pub async fn get_tip_hash(&self) -> Result<String, JsValue> {
        let url = format!("{}/blocks/tip/hash", self.base_url);
        self.fetch_text(&url).await
    }

    /// Get the current tip block height.
    pub async fn get_tip_height(&self) -> Result<u32, JsValue> {
        let url = format!("{}/blocks/tip/height", self.base_url);
        let text = self.fetch_text(&url).await?;
        text.parse::<u32>()
            .map_err(|e| JsValue::from_str(&format!("Failed to parse height: {}", e)))
    }

    /// Get block header data by hash.
    pub async fn get_block(&self, hash: &str) -> Result<JsValue, JsValue> {
        let url = format!("{}/block/{}", self.base_url, hash);
        self.fetch_json(&url).await
    }

    /// Get the current difficulty adjustment data.
    pub async fn get_difficulty_adjustment(&self) -> Result<JsValue, JsValue> {
        let url = format!("{}/v1/difficulty-adjustment", self.base_url);
        self.fetch_json(&url).await
    }

    /// Submit a raw transaction or block.
    pub async fn submit_tx(&self, hex: &str) -> Result<String, JsValue> {
        let url = format!("{}/tx", self.base_url);
        self.post_text(&url, hex).await
    }

    /// Get the network name.
    #[wasm_bindgen(getter)]
    pub fn network(&self) -> String {
        self.network.clone()
    }

    /// Get the base URL.
    #[wasm_bindgen(getter)]
    pub fn base_url(&self) -> String {
        self.base_url.clone()
    }

    /// Fetch text from a URL.
    async fn fetch_text(&self, url: &str) -> Result<String, JsValue> {
        let opts = RequestInit::new();
        opts.set_method("GET");
        opts.set_mode(RequestMode::Cors);

        let request = Request::new_with_str_and_init(url, &opts)?;

        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
        let resp: Response = resp_value.dyn_into()?;

        if !resp.ok() {
            return Err(JsValue::from_str(&format!(
                "HTTP error: {}",
                resp.status()
            )));
        }

        let text = JsFuture::from(resp.text()?).await?;
        text.as_string()
            .ok_or_else(|| JsValue::from_str("Response is not a string"))
    }

    /// Fetch JSON from a URL.
    async fn fetch_json(&self, url: &str) -> Result<JsValue, JsValue> {
        let opts = RequestInit::new();
        opts.set_method("GET");
        opts.set_mode(RequestMode::Cors);

        let request = Request::new_with_str_and_init(url, &opts)?;

        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
        let resp: Response = resp_value.dyn_into()?;

        if !resp.ok() {
            return Err(JsValue::from_str(&format!(
                "HTTP error: {}",
                resp.status()
            )));
        }

        JsFuture::from(resp.json()?).await
    }

    /// POST text to a URL.
    async fn post_text(&self, url: &str, body: &str) -> Result<String, JsValue> {
        let opts = RequestInit::new();
        opts.set_method("POST");
        opts.set_mode(RequestMode::Cors);
        opts.set_body(&JsValue::from_str(body));

        let request = Request::new_with_str_and_init(url, &opts)?;
        request.headers().set("Content-Type", "text/plain")?;

        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
        let resp: Response = resp_value.dyn_into()?;

        let text = JsFuture::from(resp.text()?).await?;
        text.as_string()
            .ok_or_else(|| JsValue::from_str("Response is not a string"))
    }
}

/// Data from the block API response.
#[derive(serde::Deserialize, Debug)]
pub struct BlockData {
    pub id: String,
    pub height: u32,
    pub version: i32,
    pub timestamp: u32,
    pub bits: u32,
    pub nonce: u32,
    pub difficulty: f64,
    pub merkle_root: String,
    pub previousblockhash: String,
}

/// Parse block data from JS value.
pub fn parse_block_data(js_value: &JsValue) -> Result<BlockData, String> {
    serde_wasm_bindgen::from_value(js_value.clone())
        .map_err(|e| format!("Failed to parse block data: {:?}", e))
}

/// Data from the difficulty adjustment API response.
#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DifficultyAdjustment {
    pub progress_percent: f64,
    pub difficulty_change: f64,
    pub estimated_retarget_date: u64,
    pub remaining_blocks: u32,
    pub remaining_time: u64,
    pub previous_retarget: f64,
    pub next_retarget_height: u32,
    pub time_avg: u64,
    pub time_offset: i64,
}

/// Parse difficulty adjustment data from JS value.
pub fn parse_difficulty_adjustment(js_value: &JsValue) -> Result<DifficultyAdjustment, String> {
    serde_wasm_bindgen::from_value(js_value.clone())
        .map_err(|e| format!("Failed to parse difficulty data: {:?}", e))
}
