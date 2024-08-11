use std::{borrow::Cow, sync::Arc, time::Duration};

use chaincash_offchain::{
    boxes::{Note, ReserveBoxSpec},
    note_history::{NoteHistory, NoteHistoryError, OwnershipEntry},
};
use chaincash_store::scans::ScanType;
use ergo_client::node::{
    endpoints::scan::{RegisteredScan, Scan, TrackingRule},
    NodeClient, NodeError,
};
use ergo_lib::{
    chain::transaction::{ergo_transaction::ErgoTransaction, Transaction, TxId},
    ergo_chain_types::EcPoint,
    ergotree_ir::{
        chain::{
            address::Address,
            ergo_box::{ErgoBox, NonMandatoryRegisterId, RegisterId},
            token::TokenId,
        },
        ergo_tree::ErgoTree,
        mir::constant::TryExtractInto,
        serialization::SigmaSerializable,
    },
};
use thiserror::Error;
use tracing::{info, warn};

use crate::ServerState;

#[derive(Error, Debug)]
pub enum ScannerError {
    #[error("Node error {0}")]
    NodeError(#[from] NodeError),
    #[error("Box error {0}")]
    BoxError(#[from] chaincash_offchain::boxes::Error),
    #[error("Store error {0}")]
    StoreError(#[from] chaincash_store::Error),
    #[error("Note history error {0}")]
    NoteHistoryError(#[from] chaincash_offchain::note_history::NoteHistoryError),
    #[error("Data input not found for TX {0}")]
    InvalidTransaction(TxId),
    #[error("Note {0:?} validation failed at TX id: {1}, reserve contract invalid")]
    InvalidReserveBox(TokenId, TxId),
}

// Wait for the next block before re-checking scans
async fn wait_scan_block(state: &ServerState) -> Result<(), ScannerError> {
    let wallet = state.node.endpoints().wallet()?;
    let cur_wallet_height = wallet.status().await?.wallet_height;
    while wallet.status().await?.wallet_height == cur_wallet_height {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    Ok(())
}

async fn get_transaction(
    node: &NodeClient,
    tx_id: &TxId,
) -> Result<Option<Transaction>, NodeError> {
    let indexer = node.endpoints().blockchain()?;
    let blocks_endpoint = node.endpoints().blocks()?;
    Ok(blocks_endpoint
        .transactions(&indexer.get_transaction_by_id(tx_id).await?.block_id)
        .await?
        .into_iter()
        .find(|tx| tx.id() == *tx_id))
}

fn pubkey_scan<'a>(
    scan_name: impl Into<std::borrow::Cow<'a, str>>,
    contract: &ErgoTree,
    pubkey_register: NonMandatoryRegisterId,
    public_keys: &[EcPoint],
) -> Scan<'a> {
    Scan {
        scan_name: scan_name.into(),
        wallet_interaction: "off".into(),
        tracking_rule: TrackingRule::And {
            args: vec![
                TrackingRule::Contains {
                    register: Some(RegisterId::R1),
                    value: contract.sigma_serialize_bytes().unwrap().into(),
                },
                TrackingRule::Or {
                    args: public_keys
                        .iter()
                        .map(|pubkey| TrackingRule::Equals {
                            register: Some(pubkey_register.into()),
                            value: pubkey.clone().into(),
                        })
                        .collect(),
                },
            ],
        },
        remove_offchain: true,
    }
}

fn pubkeys_from_scan(scan: &RegisteredScan) -> Option<Vec<EcPoint>> {
    match &scan.scan.tracking_rule {
        TrackingRule::And { args } => match &args[..] {
            [.., TrackingRule::Or { args }] => Some(
                args.iter()
                    .filter_map(|arg| {
                        if let TrackingRule::Equals { register: _, value } = arg {
                            value.clone().try_extract_into::<EcPoint>().ok()
                        } else {
                            None
                        }
                    })
                    .collect(),
            ),
            _ => None,
        },
        _ => None,
    }
}

async fn register_scan<'a>(
    state: &ServerState,
    scan_type: ScanType,
    public_keys: &[EcPoint],
) -> Result<chaincash_store::scans::Scan<'a>, ScannerError> {
    let name = format!("Chaincash {} scan", scan_type.to_str());
    let (contract, register) = match scan_type {
        ScanType::Reserves => (
            state.compiler.reserve_contract().await?,
            NonMandatoryRegisterId::R4,
        ),
        ScanType::Notes => (
            state.compiler.note_contract().await?,
            NonMandatoryRegisterId::R5,
        ),
        ScanType::Receipts => (
            state.compiler.receipt_contract().await?,
            NonMandatoryRegisterId::R7,
        ),
    };
    let scan = pubkey_scan(name, contract, register, public_keys);
    let scan_id = state.node.endpoints().scan()?.register(&scan).await?;
    let store_scan = chaincash_store::scans::Scan::new(scan_id, scan.scan_name, scan_type);
    state.store.scans().add(&store_scan)?;
    Ok(store_scan)
}

// Load scans by type. If node changes then wrong scans will be detected and re-registered
// Returns (needs_rescan, scan_type)
async fn load_scan(
    state: &ServerState,
    scan_type: ScanType,
    node_scans: &[RegisteredScan<'_>],
) -> Result<(bool, i32), ScannerError> {
    let addresses = state
        .node
        .endpoints()
        .wallet()?
        .get_addresses()
        .await?
        .into_iter()
        .filter_map(|addr| match addr.address() {
            Address::P2Pk(pk) => Some((*pk.h).clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let scan = state.store.scans().scan_by_type(scan_type)?;
    if let Some(scan) = scan {
        let node_scan = node_scans
            .iter()
            .find(|node_scan| scan.scan_name == node_scan.scan.scan_name);
        if let Some(reserve_scan) = node_scan {
            if let Some(scan_pubkeys) = pubkeys_from_scan(reserve_scan) {
                if addresses.iter().all(|wallet_pubkey| {
                    scan_pubkeys
                        .iter()
                        .any(|scan_pubkey| wallet_pubkey == scan_pubkey)
                }) {
                    return Ok((false, scan.scan_id));
                }
            } else {
                warn!(
                    "Scan #{} ({}) invalidated, re-registering",
                    scan.scan_id, scan.scan_name
                );
            }
        }
    }
    let scan_id = register_scan(state, scan_type, &addresses).await?.scan_id;
    Ok((true, scan_id))
}

async fn reserve_scanner(state: Arc<ServerState>, scan_id: i32) -> Result<(), ScannerError> {
    loop {
        let scan_boxes = state
            .node
            .extensions()
            .get_all_unspent_boxes(scan_id as u32, false)
            .await?;
        for scan_box in scan_boxes {
            match ReserveBoxSpec::try_from(&scan_box.ergo_box) {
                Ok(reserve_box) => {
                    state.store.reserves().add_or_update(&reserve_box)?;
                }
                Err(e) => warn!(
                    "Failed to import box {} from scan, err: {e}",
                    scan_box.ergo_box.box_id()
                ),
            }
        }
        wait_scan_block(&state).await?;
    }
}

async fn note_backward_scan(state: &ServerState, note_box: ErgoBox) -> Result<Note, ScannerError> {
    let indexer = &state.node.endpoints().blockchain()?;
    let note_token_id = note_box.tokens.as_ref().unwrap().first().token_id;
    let mut history = Vec::new();
    let mut cur_box = Cow::Borrowed(&note_box);
    'outer: loop {
        if let Some((id, old_note)) = state.store.notes().get_by_box_id(&cur_box.box_id())? {
            history.extend(old_note.history.ownership_entries().iter().rev().cloned());
            state.store.notes().delete_note(id)?;
            break;
        }
        let tx = get_transaction(&state.node, &cur_box.transaction_id)
            .await?
            .unwrap();
        if TokenId::from(tx.inputs.first().box_id) == note_token_id {
            // Found transaction where token was minted. Verify all tokens were sent to note contract
            let output_count = tx
                .outputs()
                .iter()
                .flat_map(|output| output.tokens.as_ref().into_iter().flatten())
                .filter(|t| t.token_id == note_token_id)
                .count();
            if output_count == 1 {
                break;
            }
        }
        let Some(reserve_input) = tx.data_inputs().map(|di| &di[0]) else {
            return Err(ScannerError::InvalidTransaction(tx.id()));
        };
        let reserve_box = ReserveBoxSpec::try_from(
            &indexer.get_box_by_id(&reserve_input.box_id).await?.ergo_box,
        )?;
        if reserve_box.ergo_box().ergo_tree != *state.compiler.reserve_contract().await? {
            return Err(ScannerError::InvalidReserveBox(note_token_id, tx.id()));
        }
        for input in tx.inputs.iter() {
            let input_box = indexer.get_box_by_id(&input.box_id).await?;
            if let Some(token) = input_box
                .ergo_box
                .tokens
                .as_ref()
                .map(|tokens| tokens.first())
            {
                if token.token_id == note_token_id
                    && input_box.ergo_box.ergo_tree == *state.compiler.note_contract().await?
                {
                    let ownership_entry = OwnershipEntry::from_context_extension(
                        *token.amount.as_u64(),
                        reserve_box.identifier,
                        &input.spending_proof.extension,
                    )?;
                    history.push(ownership_entry);
                    cur_box = Cow::Owned(input_box.ergo_box);
                    continue 'outer;
                }
            }
        }
        return Err(ScannerError::InvalidTransaction(tx.id()));
    }
    let note_history = history.into_iter().rev().try_fold(
        NoteHistory::new(),
        |mut history, entry| -> Result<NoteHistory, NoteHistoryError> {
            history.add_commitment(entry)?;
            Ok(history)
        },
    )?;
    let note = Note::new(note_box, note_history)?;
    info!(
        "Added note box id {}, identifier: {:?}",
        note.ergo_box().box_id(),
        note.note_id
    );
    Ok(note)
}

async fn note_scanner(state: Arc<ServerState>, scan_id: i32) -> Result<(), ScannerError> {
    loop {
        let scan_boxes = state
            .node
            .extensions()
            .get_all_unspent_boxes(scan_id as u32, false)
            .await
            .unwrap();
        for scan_box in &scan_boxes {
            let box_id = scan_box.ergo_box.box_id();
            if state
                .store
                .ergo_boxes()
                .get_by_id(box_id)
                .unwrap()
                .is_some()
            {
                info!("Skipping box {}", scan_box.ergo_box.box_id());
                continue;
            }
            match note_backward_scan(&state, scan_box.ergo_box.clone()).await {
                Ok(note) => {
                    state.store.notes().add_note(&note).unwrap();
                }
                Err(e) => warn!(
                    "Filtered invalid note box id {} from scan, error {e:?}",
                    box_id
                ),
            }
        }
        state
            .store
            .notes()
            .delete_not_in(scan_boxes.iter().map(|b| b.ergo_box.box_id()))?
            .into_iter()
            .for_each(|deleted| info!("Deleting box id: {deleted}"));
        wait_scan_block(&state).await.unwrap();
    }
}

pub async fn start_scanner(state: Arc<ServerState>) -> Result<(), ScannerError> {
    if let Err(NodeError::BadRequest(_)) =
        state.node.endpoints().blockchain()?.indexed_height().await
    {
        panic!("/blockchain/indexedHeight failed. Please enable extra indexing: https://docs.ergoplatform.com/node/conf/conf-node/#extra-index");
    };
    let scans = state.node.endpoints().scan()?.list_all().await?;
    let (mut needs_rescan, reserve_scan) = load_scan(&state, ScanType::Reserves, &scans).await?;
    let (rescan, note_scan) = load_scan(&state, ScanType::Notes, &scans).await?;
    needs_rescan |= rescan;
    if needs_rescan {
        //Rescan from block #1,100,000. This height can be increased later when chaincash is deployed
        let _ = state.node.endpoints().wallet()?.rescan(1_100_000).await;
    }
    tokio::spawn(reserve_scanner(state.clone(), reserve_scan));
    tokio::spawn(note_scanner(state.clone(), note_scan));
    Ok(())
}
