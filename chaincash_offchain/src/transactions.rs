pub mod reserves;

use self::reserves::{mint_reserve_transaction, MintReserveOpt};
use crate::NanoErg;
use ergo_lib::chain::{ergo_box::box_builder::ErgoBoxCandidateBuilderError, transaction::TxId};
use ergo_lib::ergo_chain_types::EcPoint;
use ergo_lib::ergotree_ir::chain::address::AddressEncoderError;
use ergo_lib::ergotree_ir::chain::ergo_box::{box_value::BoxValueError, ErgoBox};
use ergo_lib::ergotree_ir::chain::token::TokenAmountError;
use ergo_lib::wallet::box_selector::{
    BoxSelection, BoxSelector, BoxSelectorError, SimpleBoxSelector,
};
use ergo_lib::wallet::tx_builder::{TxBuilderError, SUGGESTED_TX_FEE};
use ergo_node_interface::NodeInterface;
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
}

pub struct TxContext {
    current_height: u32,
    change_address: String,
    fee: NanoErg,
}

#[derive(Clone)]
pub struct TransactionService {
    node: NodeInterface,
    fee: NanoErg,
}

impl TransactionService {
    pub fn new(node: NodeInterface) -> Self {
        Self::with_fee(node, *SUGGESTED_TX_FEE().as_u64())
    }

    pub fn with_fee(node: NodeInterface, fee: NanoErg) -> Self {
        Self { node, fee }
    }

    fn box_selection_with_amount(
        &self,
        amount: NanoErg,
    ) -> Result<BoxSelection<ErgoBox>, crate::Error> {
        let inputs = self.node.unspent_boxes_with_min_total(amount)?;
        // kinda irrelevant since we already have suitable boxes but box selectors required by ergo-lib txbuilder
        Ok(SimpleBoxSelector::new()
            .select(
                inputs,
                amount.try_into().map_err(TransactionError::from)?,
                &[],
            )
            .map_err(TransactionError::from)?)
    }

    fn get_tx_ctx(&self) -> Result<TxContext, crate::Error> {
        let wallet_status = self.node.wallet_status()?;

        // TODO: handle wallet uninitialized/locked

        Ok(TxContext {
            current_height: self.node.current_block_height()? as u32,
            change_address: wallet_status
                .change_address
                .ok_or(TransactionError::ChangeAddress("not set".to_string()))?,
            fee: self.fee,
        })
    }

    // todo should we just accept a p2pk address since it's easier for users and extract the ec point
    // ourselves?
    pub fn mint_reserve(&self, pk: EcPoint, amount: NanoErg) -> Result<TxId, crate::Error> {
        let ctx = self.get_tx_ctx()?;
        let selected_inputs = self.box_selection_with_amount(amount + ctx.fee)?;
        let unsigned_tx =
            mint_reserve_transaction(MintReserveOpt { pk, amount }, selected_inputs, ctx)?;
        Ok(self.node.sign_and_submit_transaction(&unsigned_tx)?)
    }
}
