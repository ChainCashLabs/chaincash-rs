use std::{sync::Arc, time::Duration};

use chaincash_offchain::boxes::ReserveBoxSpec;
use chaincash_store::{scans::ScanType, ChainCashStore};
use ergo_client::node::{
    endpoints::scan::{RegisteredScan, Scan, TrackingRule},
    NodeClient,
};
use ergo_lib::{
    ergo_chain_types::EcPoint,
    ergotree_ir::{
        chain::{
            address::Address,
            ergo_box::{NonMandatoryRegisterId, RegisterId},
        },
        ergo_tree::ErgoTree,
        mir::constant::TryExtractInto,
        serialization::SigmaSerializable,
    },
};
use tracing::warn;

use crate::{transaction::TransactionServiceError, ServerState};

// Wait for the next block before re-checking scans
async fn wait_scan_block(state: &ServerState) -> Result<(), TransactionServiceError> {
    let wallet = state.node.endpoints().wallet()?;
    let cur_wallet_height = wallet.status().await?.wallet_height;
    while wallet.status().await?.wallet_height == cur_wallet_height {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    Ok(())
}

fn reserve_scan(reserve_contract: &ErgoTree, public_keys: &[EcPoint]) -> Scan<'static> {
    Scan {
        scan_name: format!("Chaincash Reserve Scan").into(),
        wallet_interaction: "off".into(),
        tracking_rule: TrackingRule::And {
            args: vec![
                TrackingRule::Contains {
                    register: Some(RegisterId::R1),
                    value: reserve_contract.sigma_serialize_bytes().unwrap().into(),
                },
                TrackingRule::Or {
                    args: public_keys
                        .iter()
                        .map(|pubkey| TrackingRule::Equals {
                            register: Some(NonMandatoryRegisterId::R4.into()),
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

async fn register_reserve_scan(
    node: &NodeClient,
    store: &ChainCashStore,
    reserve_contract: &ErgoTree,
    public_keys: &[EcPoint],
) -> Result<chaincash_store::scans::Scan<'static>, TransactionServiceError> {
    let scan = reserve_scan(reserve_contract, public_keys);
    let scan_id = node.endpoints().scan()?.register(&scan).await?;
    let store_scan = chaincash_store::scans::Scan::new(scan_id, scan.scan_name, ScanType::Reserves);
    store.scans().add(&store_scan)?;
    Ok(store_scan)
}

// Load reserve scans. If node changes then wrong scans will be detected and re-registered
async fn load_reserve_scan(
    state: &ServerState,
    node_scans: &[RegisteredScan<'_>],
) -> Result<i32, TransactionServiceError> {
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
    let scan = state
        .store
        .scans()
        .scan_by_type(chaincash_store::scans::ScanType::Reserves)?;
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
                    return Ok(scan.scan_id);
                }
            } else {
                warn!(
                    "Scan #{} ({}) invalidated, re-registering",
                    scan.scan_id, scan.scan_name
                );
            }
        }
    }
    let scan_id = register_reserve_scan(
        &state.node,
        &state.store,
        state.compiler.reserve_contract().await?,
        &addresses,
    )
    .await?
    .scan_id;
    //Rescan from block #1,000,000. This height can be increased later when chaincash is deployed
    let _ = state.node.endpoints().wallet()?.rescan(1_000_000).await;
    Ok(scan_id)
}

async fn reserve_scanner(
    state: Arc<ServerState>,
    scan_id: i32,
) -> Result<(), TransactionServiceError> {
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
pub async fn start_scanner(state: Arc<ServerState>) -> Result<(), TransactionServiceError> {
    let scans = state.node.endpoints().scan()?.list_all().await?;
    tokio::spawn(reserve_scanner(
        state.clone(),
        load_reserve_scan(&state, &scans).await?,
    ));
    Ok(())
}
