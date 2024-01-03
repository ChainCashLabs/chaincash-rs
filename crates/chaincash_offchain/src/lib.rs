pub mod boxes;
pub mod contracts;
pub mod node;
pub mod transactions;

use thiserror::Error;

pub use ergo_client::Error as ClientError;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Ergo client error: {0}")]
    Client(#[from] ergo_client::Error),

    #[error("Ergo Node client error")]
    Node(#[from] ergo_client::node::NodeError),

    #[error("Transaction error occurred")]
    Transaction(#[from] transactions::TransactionError),
}
