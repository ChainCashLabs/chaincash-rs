use std::sync::Arc;

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chaincash_offchain::transactions::reserves::{MintReserveRequest, SignedReserveResponse};
use chaincash_services::transaction::TopUpReserveRequest;
use chaincash_services::ServerState;
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
    // store the reserver now or when it hits the chain? we probably need to track the box?
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

async fn list_reserves(State(state): State<Arc<ServerState>>) -> Result<Response, ApiError> {
    Ok(Json(state.store.reserves().reserve_boxes()?).into_response())
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/mint", post(mint_reserve))
        .route("/topup", post(top_up_reserve))
        .route("/", get(list_reserves))
}
