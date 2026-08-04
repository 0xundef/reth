use crate::block_trace::event::{CallKind, CallNode, CallTree};
use alloy_primitives::{Address, U256};

/// Config for the structural "trivial tree" filter.
#[derive(Debug, Clone)]
pub struct FilterConfig {
    /// Drop depth-1 trees with no internal activity. Default true.
    pub drop_trivial: bool,
    /// Selectors (hex `0x...`) that always pass the filter.
    pub keep_selectors: Vec<String>,
    /// Addresses that always pass the filter.
    pub keep_addresses: Vec<Address>,
}

/// Returns whether a tree should be forwarded to the matcher.
///
/// A *trivial* tree is a single flat call (no internal calls, no delegatecall,
/// zero value) that is fully described by the raw transaction — the matcher
/// gains nothing from it. This is a structural test, not a contract watchlist:
/// any tree with internal activity survives regardless of which contracts are
/// involved, so there is no "unlisted contract" blind spot.
pub fn should_emit(tree: &CallTree, cfg: &FilterConfig) -> bool {
    if !cfg.drop_trivial {
        return true;
    }
    let root = &tree.root;
    if root.value != U256::ZERO {
        return true;
    }
    if let Some(sel) = &root.selector {
        if cfg.keep_selectors.contains(sel) {
            return true;
        }
    }
    if cfg.keep_addresses.contains(&root.to) {
        return true;
    }
    if !root.calls.is_empty() || contains_kind(root, CallKind::DelegateCall) {
        return true;
    }
    false
}

fn contains_kind(node: &CallNode, kind: CallKind) -> bool {
    node.kind == kind || node.calls.iter().any(|c| contains_kind(c, kind))
}
