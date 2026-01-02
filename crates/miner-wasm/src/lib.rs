//! WebAssembly bindings for the Bitcoin scratch-off miner.
//!
//! This crate provides JavaScript-accessible APIs for:
//! - Fetching blockchain data from public APIs
//! - Building block templates
//! - Mining with share detection
//! - Submitting valid blocks

use wasm_bindgen::prelude::*;

pub mod api;
pub mod miner;
pub mod state;

// Re-export main types for JS access
pub use api::BlockchainApi;
pub use miner::Miner;

/// Initialize the WASM module with better panic messages.
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Get the library version.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
