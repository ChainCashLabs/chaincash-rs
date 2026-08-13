//! Background settlement of refunds whose waiting period has elapsed.
//!
//! Initiating a refund only announces it on chain; the funds stay in the reserve until the
//! notification period is over, and someone has to spend the box again to take them out. That
//! second step is what this watcher does, once per block, so an operator does not have to come
//! back days later to claim their own money.
//!
//! What gets settled is decided from the reserve boxes rather than from our own records: the
//! announcement lives in R5/R6 of the box, so a refund announced before this server existed, or
//! with the database since recreated, is picked up all the same.

use std::{sync::Arc, time::Duration};

use chaincash_offchain::transactions::reserves::refund_unlock_height;
use ergo_client::node::NodeError;
use thiserror::Error;
use tracing::{info, warn};

use crate::{transaction::TransactionServiceError, ServerState};

#[derive(Error, Debug)]
pub enum RefundError {
    #[error("Node error {0}")]
    Node(#[from] NodeError),
    #[error("Store error {0}")]
    Store(#[from] chaincash_store::Error),
    #[error("Transaction service error {0}")]
    TransactionService(#[from] TransactionServiceError),
}

/// How long to wait before retrying after a pass failed outright.
const RETRY_DELAY: Duration = Duration::from_secs(30);

/// Complete every refund of ours that is past its waiting period.
///
/// Only reserves the wallet holds the key for are considered - the reserve scan tracks every
/// reserve on the network, and we could not sign for the others anyway.
///
/// Failures are per refund and only logged: a refund that cannot be settled right now - the
/// reserve box has not been re-scanned yet, the node is busy - is retried on the next block and
/// must not hold up the others.
async fn complete_unlocked_refunds(state: &ServerState) -> Result<(), RefundError> {
    let height = state.node.endpoints().root()?.info().await?.full_height as u32;
    let reserves = state
        .store
        .reserves()
        .reserve_boxes_by_pubkeys(&state.wallet_pubkeys().await?)?;
    for reserve in reserves {
        let Some(refund_height) = reserve.refund_height else {
            continue;
        };
        if height < refund_unlock_height(refund_height) {
            continue;
        }
        match state
            .tx_service()
            .complete_refund(&reserve.identifier)
            .await
        {
            Ok(response) => info!(
                "Completed refund on reserve {}, withdrew {} nanoERG in tx {}",
                String::from(reserve.identifier),
                response.refund.withdrawn_amount.unwrap_or_default(),
                String::from(response.transaction.id()),
            ),
            Err(e) => warn!(
                "Failed to complete refund on reserve {}: {e}",
                String::from(reserve.identifier)
            ),
        }
    }
    Ok(())
}

async fn refund_watcher(state: Arc<ServerState>) {
    loop {
        if let Err(e) = complete_unlocked_refunds(&state).await {
            warn!("Refund watcher failed: {e}, retrying in {RETRY_DELAY:?}");
            tokio::time::sleep(RETRY_DELAY).await;
            continue;
        }
        if let Err(e) = crate::scanner::wait_scan_block(&state).await {
            warn!("Refund watcher failed waiting for a block: {e}");
            tokio::time::sleep(RETRY_DELAY).await;
        }
    }
}

/// Start settling refunds in the background, one pass per block.
pub fn start_refund_watcher(state: Arc<ServerState>) {
    tokio::spawn(refund_watcher(state));
}
