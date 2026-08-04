pub mod event;
pub mod filter;
pub mod inspector;
pub mod ws;

pub use event::{CallNode, CallKind, CallTree, TraceEvent, TxTraceCtx};
pub use filter::{should_emit, FilterConfig};
pub use inspector::CallTreeInspector;

use reth_tasks::Runtime;
use std::time::Duration;
use tokio::sync::mpsc;

/// Handle the engine validator uses to stream block call trees.
#[derive(Debug, Clone)]
pub struct BlockTraceHandle {
    pub sender: mpsc::Sender<TraceEvent>,
    pub filter: FilterConfig,
}

/// Spawns the WS push task and returns the sender for the execution path.
pub fn spawn_ws_task(
    runtime: &Runtime,
    url: String,
    reconnect: Duration,
    filter: FilterConfig,
) -> (mpsc::Sender<TraceEvent>, tokio::task::JoinHandle<()>) {
    let (tx, rx) = mpsc::channel(4096);
    let handle = runtime.spawn_critical_task("block-trace-ws", async move {
        ws::ws_push_loop(
            url,
            reconnect,
            rx,
            filter,
            || tracing::debug!(target: "engine::block_trace", "emitted tree"),
            || tracing::debug!(target: "engine::block_trace", "dropped tree"),
        )
        .await
    });
    (tx, handle)
}
