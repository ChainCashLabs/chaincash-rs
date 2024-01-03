pub mod notes;
pub mod reserves;

use self::notes::{mint_note_transaction, MintNoteRequest};
use self::reserves::{mint_reserve_transaction, MintReserveRequest};
use crate::contracts::{NOTE_CONTRACT, RECEIPT_CONTRACT, RESERVE_CONTRACT};
use ergo_client::node::NodeClient;
use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilderError;
use ergo_lib::ergo_chain_types::blake2b256_hash;
use ergo_lib::ergotree_ir::chain::address::AddressEncoderError;
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::{box_value::BoxValueError, ErgoBox};
use ergo_lib::ergotree_ir::chain::token::TokenAmountError;
use ergo_lib::ergotree_ir::serialization::SigmaSerializable;
use ergo_lib::wallet::box_selector::{
    BoxSelection, BoxSelector, BoxSelectorError, SimpleBoxSelector,
};
use ergo_lib::wallet::tx_builder::{TxBuilderError, SUGGESTED_TX_FEE};
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
    current_height: u32,
    change_address: String,
    fee: u64,
}
