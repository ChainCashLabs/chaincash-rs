pub mod notes;
pub mod reserves;

use ergo_lib::{
    chain::ergo_box::box_builder::ErgoBoxCandidateBuilderError,
    ergotree_ir::chain::{
        address::AddressEncoderError, ergo_box::box_value::BoxValueError, token::TokenAmountError,
    },
    wallet::{box_selector::BoxSelectorError, tx_builder::TxBuilderError},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransactionError {
    #[error("wallet change address error: {0}")]
    ChangeAddress(String),

    #[error("box value error: {0}")]
    BoxValue(#[from] BoxValueError),

    #[error("token value error: {0}")]
    TokenValue(#[from] TokenAmountError),

    #[error("missing box: {0}")]
    MissingBox(String),

    #[error("box builder error: {0}")]
    BoxBuilder(#[from] ErgoBoxCandidateBuilderError),

    #[error("box selection error: {0}")]
    BoxSelection(#[from] BoxSelectorError),

    #[error("tx builder error: {0}")]
    TxBuilder(#[from] TxBuilderError),

    #[error("address error: {0}")]
    Address(#[from] AddressEncoderError),

    #[error("parsing error: {0}")]
    Parsing(String),
}

pub struct TxContext {
    pub current_height: u32,
    pub change_address: String,
    pub fee: u64,
}
