pub mod reserves;

use crate::node::NodeInterface;

struct TransactionBuilder {
    // TODO: inputs cache so we dont need to fetch after each tx build
    node: NodeInterface,
}

impl TransactionBuilder {
    pub fn new(node: NodeInterface) -> Self {
        Self { node }
    }

    pub fn create_reserve() {
        // fetch inputs (and cache?)
        todo!()
    }
}
