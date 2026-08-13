use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chaincash_offchain::transactions::reserves::{MintReserveRequest, SignedReserveResponse};
use chaincash_services::transaction::{
    InitRefundRequest, RefundRequest, RefundResponse, TopUpReserveRequest,
};
use chaincash_services::ServerState;
use chaincash_store::refunds::RefundInfo;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use serde::Deserialize;
use serde_json::json;

use crate::api::ApiError;

async fn mint_reserve(
    State(state): State<Arc<ServerState>>,
    Json(body): Json<MintReserveRequest>,
) -> Result<Response, ApiError> {
    let SignedReserveResponse {
        reserve_box,
        transaction,
    } = state.tx_service().mint_reserve(body).await?;
    // store the reserve now or when it hits the chain? we probably need to track the box?
    let response = Json(json!({
        "txId": transaction.id(),
        "reserveNftId": reserve_box.identifier
    }));
    Ok(response.into_response())
}

async fn top_up_reserve(
    State(state): State<Arc<ServerState>>,
    Json(body): Json<TopUpReserveRequest>,
) -> Result<Response, ApiError> {
    let SignedReserveResponse {
        reserve_box: _,
        transaction,
    } = state.tx_service().top_up_reserve(body).await?;
    let response = Json(json!({
        "txId": transaction.id(),
    }));
    Ok(response.into_response())
}

async fn list_wallet_reserves(State(state): State<Arc<ServerState>>) -> Result<Response, ApiError> {
    Ok(Json(
        state
            .store
            .reserves()
            .reserve_boxes_by_pubkeys(&state.wallet_pubkeys().await?)?,
    )
    .into_response())
}

fn refund_response(response: RefundResponse) -> Response {
    Json(json!({
        "txId": response.transaction.id(),
        "refund": RefundInfo::from(response.refund),
    }))
    .into_response()
}

/// Announce a refund on a reserve.
///
/// Nothing is withdrawn yet: this starts the on-chain waiting period, during which note holders
/// can still redeem against the reserve. The server completes the refund on its own once the
/// period is over.
async fn init_refund(
    State(state): State<Arc<ServerState>>,
    Json(body): Json<InitRefundRequest>,
) -> Result<Response, ApiError> {
    Ok(refund_response(state.tx_service().init_refund(body).await?))
}

/// Call off a pending refund.
async fn cancel_refund(
    State(state): State<Arc<ServerState>>,
    Json(body): Json<RefundRequest>,
) -> Result<Response, ApiError> {
    Ok(refund_response(
        state.tx_service().cancel_refund(body).await?,
    ))
}

/// Withdraw a pending refund now. The server does this by itself when the waiting period ends;
/// this is here for when you would rather not wait for the next block to be scanned.
async fn complete_refund(
    State(state): State<Arc<ServerState>>,
    Json(body): Json<RefundRequest>,
) -> Result<Response, ApiError> {
    Ok(refund_response(
        state.tx_service().complete_refund(&body.reserve_id).await?,
    ))
}

#[derive(Deserialize)]
struct RefundsQuery {
    /// Only list refunds of this reserve
    reserve_id: Option<TokenId>,
}

/// Past and current refunds with their statuses, newest first.
async fn list_refunds(
    State(state): State<Arc<ServerState>>,
    Query(query): Query<RefundsQuery>,
) -> Result<Response, ApiError> {
    let refunds = match query.reserve_id {
        Some(reserve_id) => state.store.refunds().by_reserve(&reserve_id)?,
        None => state.store.refunds().all()?,
    };
    Ok(Json(
        refunds
            .into_iter()
            .map(RefundInfo::from)
            .collect::<Vec<_>>(),
    )
    .into_response())
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/mint", post(mint_reserve))
        .route("/topup", post(top_up_reserve))
        .route("/wallet", get(list_wallet_reserves))
        .route("/refunds", get(list_refunds))
        .route("/refund", post(init_refund))
        .route("/refund/cancel", post(cancel_refund))
        .route("/refund/complete", post(complete_refund))
}

#[cfg(test)]
mod tests {
    use axum::http::Uri;

    use super::*;

    const RESERVE_NFT: &str = "0f44aa54140dbd5368b44358630d5ca4e38e6405f76bd987e18d7eae667915db";

    fn query(uri: &str) -> Result<RefundsQuery, ()> {
        let uri: Uri = uri.parse().unwrap();
        Query::<RefundsQuery>::try_from_uri(&uri)
            .map(|Query(query)| query)
            .map_err(|_| ())
    }

    #[test]
    fn test_refunds_query() {
        assert!(query("/refunds").unwrap().reserve_id.is_none());
        assert_eq!(
            query(&format!("/refunds?reserve_id={RESERVE_NFT}"))
                .unwrap()
                .reserve_id
                .map(String::from)
                .as_deref(),
            Some(RESERVE_NFT)
        );
        assert!(query("/refunds?reserve_id=nonsense").is_err());
    }
}
