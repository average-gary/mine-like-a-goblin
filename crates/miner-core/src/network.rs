//! Bitcoin network definitions and constants.

/// Bitcoin network type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Network {
    /// Bitcoin mainnet
    Mainnet,
    /// Bitcoin testnet4
    Testnet4,
}

impl Network {
    /// Get the Bech32 human-readable part for this network.
    pub fn bech32_hrp(&self) -> &'static str {
        match self {
            Network::Mainnet => "bc",
            Network::Testnet4 => "tb",
        }
    }

    /// Get the version byte for P2PKH addresses.
    pub fn p2pkh_version(&self) -> u8 {
        match self {
            Network::Mainnet => 0x00,
            Network::Testnet4 => 0x6f,
        }
    }

    /// Get the version byte for P2SH addresses.
    pub fn p2sh_version(&self) -> u8 {
        match self {
            Network::Mainnet => 0x05,
            Network::Testnet4 => 0xc4,
        }
    }

    /// Calculate block subsidy in satoshis for a given height.
    ///
    /// The subsidy halves every 210,000 blocks, starting at 50 BTC.
    pub fn block_subsidy(&self, height: u32) -> u64 {
        let halvings = height / 210_000;
        if halvings >= 64 {
            return 0;
        }
        // 50 BTC = 5,000,000,000 satoshis
        5_000_000_000u64 >> halvings
    }

    /// Get the default RPC port for this network.
    pub fn default_rpc_port(&self) -> u16 {
        match self {
            Network::Mainnet => 8332,
            Network::Testnet4 => 48332,
        }
    }

    /// Get the mempool.space API base URL for this network.
    pub fn mempool_api_url(&self) -> &'static str {
        match self {
            Network::Mainnet => "https://mempool.space/api",
            Network::Testnet4 => "https://mempool.space/testnet4/api",
        }
    }

    /// Get the blockstream.info API base URL for this network (fallback).
    /// Note: Blockstream doesn't support testnet4, only mainnet and testnet3.
    pub fn blockstream_api_url(&self) -> Option<&'static str> {
        match self {
            Network::Mainnet => Some("https://blockstream.info/api"),
            Network::Testnet4 => None, // Not supported
        }
    }

    /// Parse network from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mainnet" | "main" | "bitcoin" => Some(Network::Mainnet),
            "testnet4" | "testnet" | "test" => Some(Network::Testnet4),
            _ => None,
        }
    }

    /// Get network name as string.
    pub fn name(&self) -> &'static str {
        match self {
            Network::Mainnet => "mainnet",
            Network::Testnet4 => "testnet4",
        }
    }

    /// Get display name for UI.
    pub fn display_name(&self) -> &'static str {
        match self {
            Network::Mainnet => "Bitcoin Mainnet",
            Network::Testnet4 => "Bitcoin Testnet4",
        }
    }
}

impl core::fmt::Display for Network {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Default for Network {
    fn default() -> Self {
        Network::Mainnet
    }
}

/// Block version with BIP9 versionbits signaling.
pub const BLOCK_VERSION: i32 = 0x20000000;

/// Size of a block header in bytes.
pub const BLOCK_HEADER_SIZE: usize = 80;

/// Coinbase maturity - blocks before coinbase can be spent.
pub const COINBASE_MATURITY: u32 = 100;

/// Maximum size of coinbase scriptSig.
pub const MAX_COINBASE_SCRIPTSIG_SIZE: usize = 100;

/// Minimum size of coinbase scriptSig (BIP34 requires at least height).
pub const MIN_COINBASE_SCRIPTSIG_SIZE: usize = 2;

/// Minimum leading zero bits required for a share (in display format).
/// 8 bits = 1 zero byte = shares display as "00..." in hex.
/// At 1 MH/s, this gives roughly 4000 shares per second (1 in 256 hashes).
pub const SHARE_MIN_LEADING_ZEROS: u32 = 8;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_subsidy() {
        let network = Network::Mainnet;

        // Genesis block: 50 BTC
        assert_eq!(network.block_subsidy(0), 5_000_000_000);

        // First halving (block 210,000): 25 BTC
        assert_eq!(network.block_subsidy(210_000), 2_500_000_000);

        // Second halving (block 420,000): 12.5 BTC
        assert_eq!(network.block_subsidy(420_000), 1_250_000_000);

        // Third halving (block 630,000): 6.25 BTC
        assert_eq!(network.block_subsidy(630_000), 625_000_000);

        // Fourth halving (block 840,000): 3.125 BTC (current era as of 2024)
        assert_eq!(network.block_subsidy(840_000), 312_500_000);
    }

    #[test]
    fn test_network_from_str() {
        assert_eq!(Network::from_str("mainnet"), Some(Network::Mainnet));
        assert_eq!(Network::from_str("MAINNET"), Some(Network::Mainnet));
        assert_eq!(Network::from_str("testnet4"), Some(Network::Testnet4));
        assert_eq!(Network::from_str("invalid"), None);
    }
}
