use crate::block_trace::event::{CallKind, CallNode, CallTree, TraceEvent};
use alloy_primitives::Address;
use revm::{
    context::ContextTr,
    inspector::Inspector,
    interpreter::{
        CallInputs, CallOutcome, CallScheme, CreateInputs, CreateOutcome, InterpreterTypes,
    },
};
use tokio::sync::mpsc;

/// Minimal call-tree inspector attached to the block import EVM.
///
/// Records every `CALL`/`CREATE` frame into a nested tree and emits one
/// [`TraceEvent::TxTree`] per transaction. System calls (pre/post execution
/// changes) go through `transact_system_call` and never reach the inspector, so
/// the top-level frame boundary coincides exactly with a transaction boundary —
/// `frame_seq` is the transaction index.
pub struct CallTreeInspector {
    sender: mpsc::Sender<TraceEvent>,
    stack: Vec<CallNode>,
    frame_seq: usize,
}

impl CallTreeInspector {
    pub fn new(sender: mpsc::Sender<TraceEvent>) -> Self {
        Self { sender, stack: Vec::new(), frame_seq: 0 }
    }

    fn finish_frame(&mut self, success: bool, gas_used: u64) {
        if let Some(mut node) = self.stack.pop() {
            node.success = success;
            node.gas_used = gas_used;
            if let Some(parent) = self.stack.last_mut() {
                parent.calls.push(node);
            } else {
                let index = self.frame_seq;
                self.frame_seq += 1;
                let _ = self.sender.try_send(TraceEvent::TxTree {
                    index,
                    tree: CallTree { root: node },
                });
            }
        }
    }
}

impl<CTX: ContextTr, INTR: InterpreterTypes> Inspector<CTX, INTR> for CallTreeInspector {
    fn call(&mut self, context: &mut CTX, inputs: &mut CallInputs) -> Option<CallOutcome> {
        let input = inputs.input.as_bytes_local(context.local());
        let selector = (input.len() >= 4).then(|| {
            format!(
                "0x{:02x}{:02x}{:02x}{:02x}",
                input[0], input[1], input[2], input[3]
            )
        });
        let kind = match inputs.scheme {
            CallScheme::Call => CallKind::Call,
            CallScheme::CallCode => CallKind::CallCode,
            CallScheme::DelegateCall => CallKind::DelegateCall,
            CallScheme::StaticCall => CallKind::StaticCall,
        };
        self.stack.push(CallNode {
            kind,
            from: inputs.caller,
            to: inputs.target_address,
            selector,
            value: inputs.value.get(),
            success: false,
            gas_used: 0,
            calls: Vec::new(),
        });
        None
    }

    fn call_end(&mut self, _context: &mut CTX, _inputs: &CallInputs, outcome: &mut CallOutcome) {
        self.finish_frame(outcome.instruction_result().is_ok(), outcome.gas().spent());
    }

    fn create(&mut self, _context: &mut CTX, inputs: &mut CreateInputs) -> Option<CreateOutcome> {
        self.stack.push(CallNode {
            kind: CallKind::Create,
            from: inputs.caller(),
            // The created address depends on the caller nonce; not resolved here.
            to: Address::ZERO,
            selector: None,
            value: inputs.value(),
            success: false,
            gas_used: 0,
            calls: Vec::new(),
        });
        None
    }

    fn create_end(
        &mut self,
        _context: &mut CTX,
        _inputs: &CreateInputs,
        outcome: &mut CreateOutcome,
    ) {
        self.finish_frame(outcome.instruction_result().is_ok(), outcome.gas().spent());
    }
}
