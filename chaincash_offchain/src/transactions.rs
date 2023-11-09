pub mod reserves;

use ergo_lib::ergo_chain_types::EcPoint;
use ergo_node_interface::NodeInterface;

#[derive(Clone)]
pub struct TransactionBuilder {
    // TODO: inputs cache so we dont need to fetch after each tx build
    node: NodeInterface,
}

impl TransactionBuilder {
    pub fn new(node: NodeInterface) -> Self {
        Self { node }
    }

    pub fn create_reserve(pk: EcPoint, amount: u64) {
        // fetch inputs (and cache?)
        todo!()
    }
}
