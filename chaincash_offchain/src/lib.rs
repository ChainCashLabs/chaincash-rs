pub mod contracts;
pub mod node;
pub mod transactions;

use ergo_node_interface::node_interface::NodeError;
use thiserror::Error;

pub use ergo_node_interface::{NanoErg, NodeInterface};
pub use transactions::TransactionService;

#[derive(Debug, Error)]
pub enum Error {
    #[error("node error: {0}")]
    Node(#[from] NodeError),

    #[error("transaction error: {0}")]
    Transaction(#[from] transactions::TransactionError),
}
