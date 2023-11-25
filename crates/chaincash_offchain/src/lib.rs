pub mod contracts;
pub mod node;
pub mod transactions;

use thiserror::Error;

pub use ergo_client::Error as ClientError;
pub use transactions::TransactionService;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Ergo client error")]
    Client(#[from] ergo_client::Error),

    #[error("Ergo Node client error")]
    Node(#[from] ergo_client::node::NodeError),

    #[error("transaction error: {0}")]
    Transaction(#[from] transactions::TransactionError),
}
