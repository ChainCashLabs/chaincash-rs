use chaincash_offchain::transactions::notes::{
    mint_note_transaction, spend_note_transaction, MintNoteRequest, MintNoteResponse,
    SignedMintNoteResponse, SignedSpendNoteResponse, SpendNoteResponse,
};
use chaincash_offchain::transactions::reserves::{
    mint_reserve_transaction, top_up_reserve_transaction, MintReserveRequest, ReserveResponse,
    SignedReserveResponse,
};
use chaincash_offchain::transactions::{TransactionError, TxContext};
use chaincash_store::ChainCashStore;
use ergo_client::node::NodeClient;
use ergo_lib::ergo_chain_types::EcPoint;
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::{box_value::BoxValueError, ErgoBox};
use ergo_lib::ergotree_ir::chain::token::{TokenAmount, TokenId};
use ergo_lib::wallet::box_selector::{
    BoxSelection, BoxSelector, BoxSelectorError, SimpleBoxSelector,
};
use ergo_lib::wallet::tx_builder::SUGGESTED_TX_FEE;
use serde::Deserialize;
use thiserror::Error;

use crate::compiler::Compiler;

#[derive(Debug, Error)]
pub enum TransactionServiceError {
    #[error("Change address not set in wallet")]
    ChangeAddressNotSet,

    #[error("An error occurred while building transaction: {0}")]
    TransactionBuilding(#[from] TransactionError),

    #[error("Failed to convert ergo boxes into 'selected' boxes for transaction: {0}")]
    BoxSelection(#[from] BoxSelectorError),

    #[error("Invalid box value supplied")]
    BoxValue(#[from] BoxValueError),

    #[error("Node operation failed: {0}")]
    Node(#[from] ergo_client::node::NodeError),

    #[error("Store error: {0}")]
    Store(#[from] chaincash_store::Error),

    #[error("Reserve Box not found")]
    ReserveBoxNotFound,
}

#[derive(Deserialize)]
pub struct SpendNoteRequest {
    /// ID of note in database
    note_id: i32,
    reserve_id: TokenId,
    recipient_pubkey: EcPoint,
    amount: TokenAmount,
}

#[derive(Deserialize)]
pub struct TopUpReserveRequest {
    /// ID of note in database
    reserve_id: TokenId,
    top_up_amount: u64,
}

#[derive(Clone)]
pub struct TransactionService<'a> {
    node: &'a NodeClient,
    compiler: &'a Compiler,
    store: &'a ChainCashStore,
}

impl<'a> TransactionService<'a> {
    pub fn new(node: &'a NodeClient, store: &'a ChainCashStore, compiler: &'a Compiler) -> Self {
        Self {
            node,
            store,
            compiler,
        }
    }

    async fn box_selection_with_amount(
        &self,
        amount: u64,
    ) -> Result<BoxSelection<ErgoBox>, TransactionServiceError> {
        let inputs = self
            .node
            .extensions()
            .get_utxos_summing_amount(amount)
            .await?;
        // kinda irrelevant since we already have suitable boxes but box selectors required by ergo-lib txbuilder
        Ok(SimpleBoxSelector::new()
            .select(
                inputs,
                amount.try_into().map_err(TransactionServiceError::from)?,
                &[],
            )
            .map_err(TransactionServiceError::from)?)
    }

    async fn get_tx_ctx(&self) -> Result<TxContext, TransactionServiceError> {
        let wallet_status = self.node.endpoints().wallet()?.status().await?;
        let info = self.node.endpoints().root()?.info().await?;

        if wallet_status.change_address.is_empty() {
            Err(TransactionServiceError::ChangeAddressNotSet)?
        } else {
            Ok(TxContext {
                current_height: info.full_height as u32,
                change_address: wallet_status.change_address,
                fee: *SUGGESTED_TX_FEE().as_u64(),
            })
        }
    }

    /// Create a mint reserve transaction and add minted reserve box to DB
    pub async fn mint_reserve(
        &self,
        request: MintReserveRequest,
    ) -> Result<SignedReserveResponse, TransactionServiceError> {
        let ctx = self.get_tx_ctx().await?;
        let selected_inputs = self
            .box_selection_with_amount(request.amount + ctx.fee)
            .await?;
        let reserve_tree = self.compiler.reserve_contract().await?.clone();
        let ReserveResponse {
            reserve_box,
            transaction,
        } = mint_reserve_transaction(request, reserve_tree, selected_inputs, ctx)?;
        let submitted_tx = self.node.extensions().sign_and_submit(transaction).await?;
        self.store.reserves().add_or_update(&reserve_box)?;
        Ok(SignedReserveResponse {
            reserve_box,
            transaction: submitted_tx,
        })
    }

    pub async fn top_up_reserve(
        &self,
        request: TopUpReserveRequest,
    ) -> Result<SignedReserveResponse, TransactionServiceError> {
        let ctx = self.get_tx_ctx().await?;
        let wallet_boxes = self.node.extensions().get_utxos().await?;
        let reserve = self
            .store
            .reserves()
            .get_reserve_by_identifier(&request.reserve_id)?;
        let ReserveResponse {
            reserve_box,
            transaction,
        } = top_up_reserve_transaction(&reserve, wallet_boxes, request.top_up_amount, &ctx)?;
        let submitted_tx = self.node.extensions().sign_and_submit(transaction).await?;
        self.store.reserves().add_or_update(&reserve_box)?;
        Ok(SignedReserveResponse {
            reserve_box,
            transaction: submitted_tx,
        })
    }

    pub async fn mint_note(
        &self,
        request: MintNoteRequest,
    ) -> Result<SignedMintNoteResponse, TransactionServiceError> {
        let ctx = self.get_tx_ctx().await?;
        let selected_inputs = self
            .box_selection_with_amount(BoxValue::SAFE_USER_MIN.as_u64() + ctx.fee)
            .await?;
        let note_tree = self.compiler.note_contract().await?.clone();
        let MintNoteResponse { note, transaction } =
            mint_note_transaction(request, note_tree, selected_inputs, ctx)?;
        let submitted_tx = self.node.extensions().sign_and_submit(transaction).await?;
        self.store.notes().add_note(&note)?;
        Ok(SignedMintNoteResponse {
            note,
            transaction: submitted_tx,
        })
    }

    pub async fn spend_note(
        &self,
        request: SpendNoteRequest,
    ) -> Result<SignedSpendNoteResponse, TransactionServiceError> {
        let note = self.store.notes().get_note_box(request.note_id)?;
        let reserve = self
            .store
            .reserves()
            .get_reserve_by_identifier(&request.reserve_id)?;
        let private_key = self
            .node
            .extensions()
            .get_private_key(note.owner.clone())
            .await?
            .w;
        let wallet_boxes = self.node.extensions().get_utxos().await?;
        let tx_context = self.get_tx_ctx().await?;
        let SpendNoteResponse {
            transaction,
            recipient_note,
            change_note,
        } = spend_note_transaction(
            &note,
            &reserve,
            private_key,
            request.recipient_pubkey,
            *request.amount.as_u64(),
            wallet_boxes,
            &tx_context,
        )?;

        let transaction = self.node.extensions().sign_and_submit(transaction).await?;
        self.store.notes().delete_note(request.note_id)?;
        if let Some(ref change_note) = change_note {
            self.store.notes().add_note(change_note)?;
        }

        Ok(SignedSpendNoteResponse {
            transaction,
            recipient_note,
            change_note,
        })
    }
}
