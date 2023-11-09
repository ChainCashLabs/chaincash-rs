pub mod reserves;

use crate::NanoErg;
use ergo_lib::{ergo_chain_types::EcPoint, wallet::tx_builder::SUGGESTED_TX_FEE};
use ergo_node_interface::NodeInterface;

#[derive(Clone)]
pub struct TransactionService {
    // TODO: inputs cache so we dont need to fetch after each tx build
    node: NodeInterface,
    fee: NanoErg,
}

struct TxContext {
    current_height: u64,
    change_address: String,
    fee: NanoErg,
}

impl TransactionService {
    pub fn new(node: NodeInterface) -> Self {
        Self::with_fee(node, SUGGESTED_TX_FEE().as_u64().clone())
    }

    pub fn with_fee(node: NodeInterface, fee: NanoErg) -> Self {
        Self { node, fee }
    }

    // TODO: handle request failures
    // handle wallet uninitialized/locked
    // handle no change address
    fn get_ctx(&self) -> TxContext {
        let wallet_status = self.node.wallet_status().unwrap();

        TxContext {
            current_height: self.node.current_block_height().unwrap(),
            change_address: wallet_status.change_address.unwrap(),
            fee: self.fee,
        }
    }

    pub fn create_reserve(&self, pk: EcPoint, amount: NanoErg) {
        // TODO: get utxos from a cache
        let ctx = self.get_ctx();
        let inputs = self
            .node
            .unspent_boxes_with_min_total(amount + ctx.fee)
            .unwrap();
        // transform inputs into box_selector
        // build reserve output ErgoboxCandidate
        // still want to use a separate function to actually build the tx so we can decouple from
        // node usage and make the building testable
    }
}
