use crate::block_trace::event::{CallTree, TraceEvent};
use crate::block_trace::filter::{should_emit, FilterConfig};
use alloy_primitives::{Address, B256};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use std::collections::VecDeque;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WsMessage {
    block_hash: B256,
    block_number: u64,
    tx_index: usize,
    tx_hash: B256,
    signer: Address,
    tree: CallTree,
}

struct Pending {
    index: usize,
    block_hash: B256,
    block_number: u64,
    tx_hash: B256,
    signer: Address,
}

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Consumes [`TraceEvent`]s and pushes call trees over WebSocket.
///
/// Best-effort: when the WS is down or the channel is full, events are dropped.
/// Trees are paired with their tx metadata by `index`, so drops never cause
/// misalignment between trees and their (block, tx) metadata.
///
/// The socket is polled concurrently with the event channel so incoming PING
/// frames are answered (keeping the connection alive) and close frames are
/// detected for reconnection.
pub async fn ws_push_loop(
    ws_url: String,
    reconnect: Duration,
    mut rx: mpsc::Receiver<TraceEvent>,
    filter: FilterConfig,
    mut on_emitted: impl FnMut(),
    mut on_dropped: impl FnMut(),
) {
    let mut pending: VecDeque<Pending> = VecDeque::new();
    let mut ws: Option<Ws> = None;

    loop {
        if ws.is_none() {
            match tokio_tungstenite::connect_async(&ws_url).await {
                Ok((stream, _)) => ws = Some(stream),
                Err(_) => {
                    tokio::time::sleep(reconnect).await;
                    continue;
                }
            }
        }

        let mut reconnect_ws = false;
        let w = ws.as_mut().expect("ws connected above");

        tokio::select! {
            evt = rx.recv() => {
                let Some(evt) = evt else { return };
                match evt {
                    TraceEvent::TxStart { block_hash, block_number, index, hash, signer } => {
                        pending.push_back(Pending { index, block_hash, block_number, tx_hash: hash, signer });
                    }
                    TraceEvent::TxTree { index, tree } => {
                        // Drop any trees whose TxStart was lost (channel full / WS down).
                        while pending.front().map(|p| p.index) != Some(index) {
                            pending.pop_front();
                            on_dropped();
                        }
                        let Some(meta) = pending.pop_front() else {
                            on_dropped();
                            continue;
                        };
                        if !should_emit(&tree, &filter) {
                            on_dropped();
                            continue;
                        }
                        let msg = WsMessage {
                            block_hash: meta.block_hash,
                            block_number: meta.block_number,
                            tx_index: meta.index,
                            tx_hash: meta.tx_hash,
                            signer: meta.signer,
                            tree,
                        };
                        let Ok(payload) = serde_json::to_vec(&msg) else { continue };
                        if w.send(Message::Binary(payload.into())).await.is_err() {
                            reconnect_ws = true;
                        } else {
                            on_emitted();
                        }
                    }
                }
            }
            incoming = w.next() => {
                // Polling the stream lets the underlying tungstenite client answer PING
                // frames automatically, keeping the connection alive. A `None` means the
                // peer closed the connection; anything else (ping/pong/text) is ignored.
                if incoming.is_none() {
                    reconnect_ws = true;
                }
            }
        }

        if reconnect_ws {
            ws = None;
        }
    }
}
