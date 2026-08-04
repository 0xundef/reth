//! clap [Args](clap::Args) for block call-tree streaming.

use alloy_primitives::Address;
use clap::Args;

/// Parameters for streaming per-transaction call trees captured during block import.
#[derive(Debug, Clone, Args, PartialEq, Eq)]
#[command(next_help_heading = "Block trace")]
pub struct BlockTraceArgs {
    /// Enable capturing call trees of imported blocks and pushing them over WebSocket.
    #[arg(long = "block-trace.enabled", default_value_t = false, help_heading = "Block trace")]
    pub enabled: bool,

    /// WebSocket URL of the matcher to push call trees to (e.g. `ws://127.0.0.1:9000/trace`).
    /// Required when `--block-trace.enabled` is set.
    #[arg(long = "block-trace.ws-url", help_heading = "Block trace")]
    pub ws_url: Option<String>,

    /// Reconnect interval (ms) for the WebSocket client.
    #[arg(
        long = "block-trace.reconnect-interval-ms",
        default_value_t = 5000,
        help_heading = "Block trace"
    )]
    pub reconnect_interval_ms: u64,

    /// Drop trees with no internal activity (depth-1 calls with no delegatecall/value).
    #[arg(long = "block-trace.drop-trivial", default_value_t = true, help_heading = "Block trace")]
    pub drop_trivial: bool,

    /// Selectors (hex `0x...`) that are always kept even when the tree is trivial.
    #[arg(long = "block-trace.keep-selector", value_delimiter = ',', help_heading = "Block trace")]
    pub keep_selectors: Vec<String>,

    /// Contract addresses that are always kept even when the tree is trivial.
    #[arg(
        long = "block-trace.keep-address",
        value_delimiter = ',',
        help_heading = "Block trace"
    )]
    pub keep_addresses: Vec<Address>,
}

impl Default for BlockTraceArgs {
    fn default() -> Self {
        Self {
            enabled: false,
            ws_url: None,
            reconnect_interval_ms: 5000,
            drop_trivial: true,
            keep_selectors: Vec::new(),
            keep_addresses: Vec::new(),
        }
    }
}
