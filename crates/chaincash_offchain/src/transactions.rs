pub mod reserves;

use crate::contracts::RESERVE_CONTRACT;

use self::reserves::{mint_reserve_transaction, MintReserveOpt};
use ergo_client::node::NodeClient;
use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilderError;
use ergo_lib::ergotree_ir::chain::address::AddressEncoderError;
use ergo_lib::ergotree_ir::chain::ergo_box::{box_value::BoxValueError, ErgoBox};
use ergo_lib::ergotree_ir::chain::token::TokenAmountError;
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

#[derive(Clone)]
pub struct TransactionService<'a> {
    node: &'a NodeClient,
}

impl<'a> TransactionService<'a> {
    pub fn new(node: &'a NodeClient) -> Self {
        Self { node }
    }

    async fn box_selection_with_amount(
        &self,
        amount: u64,
    ) -> Result<BoxSelection<ErgoBox>, crate::Error> {
        let inputs = self
            .node
            .extensions()
            .get_utxos_summing_amount(amount)
            .await?;
        // kinda irrelevant since we already have suitable boxes but box selectors required by ergo-lib txbuilder
        Ok(SimpleBoxSelector::new()
            .select(
                inputs,
                amount.try_into().map_err(TransactionError::from)?,
                &[],
            )
            .map_err(TransactionError::from)?)
    }

    async fn get_tx_ctx(&self) -> Result<TxContext, crate::Error> {
        let wallet_status = self.node.endpoints().wallet()?.status().await?;
        let info = self.node.endpoints().root()?.info().await?;

        if wallet_status.change_address == "" {
            Err(TransactionError::ChangeAddress(
                "address not set".to_owned(),
            ))?
        } else {
            Ok(TxContext {
                current_height: info.full_height as u32,
                change_address: wallet_status.change_address,
                fee: *SUGGESTED_TX_FEE().as_u64(),
            })
        }
    }

    pub async fn mint_reserve(&self, opts: MintReserveOpt) -> Result<String, crate::Error> {
        let ctx = self.get_tx_ctx().await?;
        let selected_inputs = self
            .box_selection_with_amount(opts.amount + ctx.fee)
            .await?;
        let reserve_tree = self
            .node
            .extensions()
            .compile_contract(RESERVE_CONTRACT)
            .await?;
        let unsigned_tx = mint_reserve_transaction(opts, reserve_tree, selected_inputs, ctx)?;

        Ok(self.node.extensions().sign_and_submit(unsigned_tx).await?)
    }
}
