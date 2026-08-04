use alloy_primitives::{Address, B256, U256};
use serde::Serialize;

/// Type of a call frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CallKind {
    Call,
    DelegateCall,
    StaticCall,
    CallCode,
    Create,
}

/// A single node in a call tree.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallNode {
    pub kind: CallKind,
    pub from: Address,
    pub to: Address,
    /// First 4 bytes of calldata, hex encoded (`0x6a761202`), when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
    pub value: U256,
    pub success: bool,
    pub gas_used: u64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub calls: Vec<CallNode>,
}

/// Call tree of a single transaction.
#[derive(Debug, Clone, Serialize)]
pub struct CallTree {
    pub root: CallNode,
}

/// Events flowing from the execution path to the WS push task.
#[derive(Debug)]
pub enum TraceEvent {
    /// Emitted just before a transaction executes.
    TxStart { block_hash: B256, block_number: u64, index: usize, hash: B256, signer: Address },
    /// Emitted by the inspector when a top-level frame finishes (== one tx).
    TxTree { index: usize, tree: CallTree },
}

/// Minimal context passed from the executor to the inspector.
#[derive(Clone)]
pub struct TxTraceCtx {
    pub block_hash: B256,
    pub block_number: u64,
    pub sender: tokio::sync::mpsc::Sender<TraceEvent>,
}
