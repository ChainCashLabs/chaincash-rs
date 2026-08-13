use chaincash_offchain::boxes::ReserveBoxSpec;
use chaincash_offchain::oracle::{buyback_nft, oracle_nft};
use chaincash_offchain::transactions::notes::{
    mint_note_transaction, redeem_note, spend_note_transaction, MintNoteRequest, MintNoteResponse,
    SignedMintNoteResponse, SignedSpendNoteResponse, SpendNoteResponse,
};
use chaincash_offchain::transactions::reserves::{
    cancel_refund_transaction, complete_refund_transaction, init_refund_transaction,
    mint_reserve_transaction, top_up_reserve_transaction, MintReserveRequest, ReserveResponse,
    SignedReserveResponse,
};
use chaincash_offchain::transactions::{TransactionError, TxContext};
use chaincash_store::refunds::Refund;
use chaincash_store::ChainCashStore;
use ergo_client::node::endpoints::blockchain::IndexQuery;
use ergo_client::node::NodeClient;
use ergo_lib::chain::transaction::Transaction;
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

    #[error("No pending refund recorded for reserve {0}")]
    PendingRefundNotFound(String),
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

#[derive(Deserialize)]
pub struct RedeemNoteRequest {
    note_id: i32,
    reserve_id: TokenId,
}

#[derive(Deserialize)]
pub struct InitRefundRequest {
    /// NFT ID of the reserve to refund from
    pub reserve_id: TokenId,
    /// nanoERG to withdraw once the waiting period is over
    pub amount: u64,
}

#[derive(Deserialize)]
pub struct RefundRequest {
    /// NFT ID of the reserve whose pending refund is being acted on
    pub reserve_id: TokenId,
}

/// A submitted refund transaction together with the refund record it changed.
pub struct RefundResponse {
    pub transaction: Transaction,
    pub refund: Refund,
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

    /// Announce a refund on a reserve, starting the on-chain waiting period.
    ///
    /// No funds move here: the transaction only writes the announcement into the reserve box, so
    /// note holders keep being able to redeem for the whole period. Once it is over the refund is
    /// settled automatically by the refund watcher, or on demand via [`Self::complete_refund`].
    pub async fn init_refund(
        &self,
        request: InitRefundRequest,
    ) -> Result<RefundResponse, TransactionServiceError> {
        let ctx = self.get_tx_ctx().await?;
        let wallet_boxes = self.node.extensions().get_utxos().await?;
        let reserve = self
            .store
            .reserves()
            .get_reserve_by_identifier(&request.reserve_id)?;
        let ReserveResponse {
            reserve_box,
            transaction,
        } = init_refund_transaction(&reserve, wallet_boxes, request.amount, &ctx)?;
        let transaction = self.node.extensions().sign_and_submit(transaction).await?;
        self.store.reserves().add_or_update(&reserve_box)?;
        // Record the height the contract actually got, not the one we asked for.
        let refund = self.store.refunds().add(
            &request.reserve_id,
            request.amount as i64,
            reserve_box
                .refund_height
                .unwrap_or(ctx.current_height as i32),
            Some(&String::from(transaction.id())),
        )?;
        Ok(RefundResponse {
            transaction,
            refund,
        })
    }

    /// Call off a refund before it is completed, leaving the reserve untouched.
    pub async fn cancel_refund(
        &self,
        request: RefundRequest,
    ) -> Result<RefundResponse, TransactionServiceError> {
        let ctx = self.get_tx_ctx().await?;
        let wallet_boxes = self.node.extensions().get_utxos().await?;
        let reserve = self
            .store
            .reserves()
            .get_reserve_by_identifier(&request.reserve_id)?;
        let pending = self.pending_refund(&reserve)?;
        let ReserveResponse {
            reserve_box,
            transaction,
        } = cancel_refund_transaction(&reserve, wallet_boxes, &ctx)?;
        let transaction = self.node.extensions().sign_and_submit(transaction).await?;
        self.store.reserves().add_or_update(&reserve_box)?;
        let refund = self
            .store
            .refunds()
            .cancel(pending.id, Some(&String::from(transaction.id())))?;
        Ok(RefundResponse {
            transaction,
            refund,
        })
    }

    /// Withdraw an announced refund. Fails while the waiting period is still running - both here
    /// and, were the check skipped, in the contract.
    pub async fn complete_refund(
        &self,
        reserve_id: &TokenId,
    ) -> Result<RefundResponse, TransactionServiceError> {
        let ctx = self.get_tx_ctx().await?;
        let wallet_boxes = self.node.extensions().get_utxos().await?;
        let reserve = self
            .store
            .reserves()
            .get_reserve_by_identifier(reserve_id)?;
        let pending = self.pending_refund(&reserve)?;
        let reserve_value = *reserve.ergo_box().value.as_u64();
        let ReserveResponse {
            reserve_box,
            transaction,
        } = complete_refund_transaction(&reserve, wallet_boxes, &ctx)?;
        let withdrawn = reserve_value.saturating_sub(*reserve_box.ergo_box().value.as_u64());
        let transaction = self.node.extensions().sign_and_submit(transaction).await?;
        self.store.reserves().add_or_update(&reserve_box)?;
        let refund = self.store.refunds().complete(
            pending.id,
            withdrawn as i64,
            &String::from(transaction.id()),
        )?;
        Ok(RefundResponse {
            transaction,
            refund,
        })
    }

    /// The record of the refund pending on `reserve`.
    ///
    /// The announcement lives in the reserve box, not in our database: a refund started before
    /// this server saw the reserve - or with the database since recreated - is still settleable,
    /// and should still show up in the history afterwards. So when the box says a refund is
    /// pending and we have no record of it, backfill one from the registers.
    fn pending_refund(&self, reserve: &ReserveBoxSpec) -> Result<Refund, TransactionServiceError> {
        if let Some(refund) = self
            .store
            .refunds()
            .pending_for_reserve(&reserve.identifier)?
        {
            return Ok(refund);
        }
        let (Some(refund_height), Some(refund_amount)) =
            (reserve.refund_height, reserve.refund_amount)
        else {
            return Err(TransactionServiceError::PendingRefundNotFound(
                String::from(reserve.identifier),
            ));
        };
        Ok(self
            .store
            .refunds()
            .add(&reserve.identifier, refund_amount, refund_height, None)?)
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

    pub async fn redeem_note(
        &self,
        request: RedeemNoteRequest,
    ) -> Result<Transaction, TransactionServiceError> {
        let note_box = self.store.notes().get_note_box(request.note_id)?;
        let reserve_box = self
            .store
            .reserves()
            .get_reserve_by_identifier(&request.reserve_id)?;
        let receipt_contract = self.compiler.receipt_contract().await?;
        let is_mainnet = self.node.endpoints().root()?.info().await?.network == "mainnet";
        let buyback_box = &self
            .node
            .endpoints()
            .blockchain()?
            .get_unspent_boxes_by_token_id(
                &String::from(buyback_nft(is_mainnet)),
                IndexQuery {
                    offset: 0,
                    limit: 1,
                    sort_direction:
                        ergo_client::node::endpoints::blockchain::SortDirection::Descending,
                    include_unconfirmed: true,
                },
            )
            .await?[0]
            .ergo_box;
        let wallet_boxes = self.node.extensions().get_utxos().await?;
        let tx_context = self.get_tx_ctx().await?;
        let oracle_box = &self
            .node
            .endpoints()
            .blockchain()?
            .get_unspent_boxes_by_token_id(
                &String::from(oracle_nft(is_mainnet)),
                IndexQuery {
                    offset: 0,
                    limit: 1,
                    sort_direction:
                        ergo_client::node::endpoints::blockchain::SortDirection::Descending,
                    include_unconfirmed: true,
                },
            )
            .await?[0]
            .ergo_box;
        let tx = redeem_note(
            &note_box,
            &reserve_box,
            &oracle_box,
            &buyback_box,
            receipt_contract,
            wallet_boxes,
            &tx_context,
        )?;
        let tx = self.node.extensions().sign_and_submit(tx).await?;
        Ok(tx)
    }
}
