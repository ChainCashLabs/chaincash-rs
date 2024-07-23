use chaincash_offchain::contracts::{NOTE_CONTRACT, RECEIPT_CONTRACT, RESERVE_CONTRACT};
use chaincash_offchain::transactions::notes::{
    mint_note_transaction, spend_note_transaction, MintNoteRequest, MintNoteResponse,
    SignedMintNoteResponse, SignedSpendNoteResponse, SpendNoteResponse,
};
use chaincash_offchain::transactions::reserves::{
    mint_reserve_transaction, MintReserveRequest, MintReserveResponse, SignedMintReserveResponse,
};
use chaincash_offchain::transactions::{TransactionError, TxContext};
use chaincash_store::ChainCashStore;
use ergo_client::node::NodeClient;
use ergo_lib::ergo_chain_types::{blake2b256_hash, EcPoint};
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::{BoxValue, BoxValueError};
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::token::{TokenAmount, TokenId};
use ergo_lib::ergotree_ir::serialization::SigmaSerializable;
use ergo_lib::wallet::box_selector::{
    BoxSelection, BoxSelector, BoxSelectorError, SimpleBoxSelector,
};
use ergo_lib::wallet::tx_builder::SUGGESTED_TX_FEE;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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

#[derive(Serialize, Deserialize)]
pub struct SpendNoteRequest {
    /// ID of note in database
    note_id: i32,
    reserve_id: TokenId,
    recipient_pubkey: EcPoint,
    amount: TokenAmount,
}

#[derive(Clone)]
pub struct TransactionService<'a> {
    node: &'a NodeClient,
    store: &'a ChainCashStore,
}

impl<'a> TransactionService<'a> {
    pub fn new(node: &'a NodeClient, store: &'a ChainCashStore) -> Self {
        Self { node, store }
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
        SimpleBoxSelector::new()
            .select(
                inputs,
                amount.try_into().map_err(TransactionServiceError::from)?,
                &[],
            )
            .map_err(TransactionServiceError::from)
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
    ) -> Result<SignedMintReserveResponse, TransactionServiceError> {
        let ctx = self.get_tx_ctx().await?;
        let selected_inputs = self
            .box_selection_with_amount(request.amount + ctx.fee)
            .await?;
        let reserve_tree = self
            .node
            .extensions()
            .compile_contract(RESERVE_CONTRACT)
            .await?;
        let MintReserveResponse {
            reserve_box,
            transaction,
        } = mint_reserve_transaction(request, reserve_tree, selected_inputs, ctx)?;
        let submitted_tx = self.node.extensions().sign_and_submit(transaction).await?;
        self.store.reserves().add(&reserve_box)?;
        Ok(SignedMintReserveResponse {
            reserve_box,
            transaction: submitted_tx,
        })
    }

    pub async fn mint_note(
        &self,
        request: MintNoteRequest,
    ) -> Result<SignedMintNoteResponse, TransactionServiceError> {
        let reserve_tree_bytes = self
            .node
            .extensions()
            .compile_contract(RESERVE_CONTRACT)
            .await?
            .sigma_serialize_bytes()
            .unwrap();
        let reserve_hash = bs58::encode(blake2b256_hash(&reserve_tree_bytes[1..])).into_string();
        let receipt_contract = RECEIPT_CONTRACT.replace("$reserveContractHash", &reserve_hash);
        let receipt_tree_bytes = self
            .node
            .extensions()
            .compile_contract(&receipt_contract)
            .await?
            .sigma_serialize_bytes()
            .unwrap();
        let receipt_hash = bs58::encode(blake2b256_hash(&receipt_tree_bytes[1..])).into_string();
        let ctx = self.get_tx_ctx().await?;
        let selected_inputs = self
            .box_selection_with_amount(BoxValue::SAFE_USER_MIN.as_u64() + ctx.fee)
            .await?;
        let note_contract = NOTE_CONTRACT
            .replace("$reserveContractHash", &reserve_hash)
            .replace("$receiptContractHash", &receipt_hash);
        let contract_tree = self
            .node
            .extensions()
            .compile_contract(&note_contract)
            .await?;
        let MintNoteResponse { note, transaction } =
            mint_note_transaction(request, contract_tree, selected_inputs, ctx)?;
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
            tx_context,
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
