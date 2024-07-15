use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chaincash_offchain::transactions::reserves::{MintReserveRequest, SignedMintReserveResponse};
use serde_json::json;

use crate::api::ApiError;

async fn mint_reserve(
    State(state): State<crate::ServerState>,
    Json(body): Json<MintReserveRequest>,
) -> Result<Response, ApiError> {
    let SignedMintReserveResponse {
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

async fn list_reserves(State(state): State<crate::ServerState>) -> Result<Response, ApiError> {
    Ok(Json(state.store.reserves().reserve_boxes()?).into_response())
}

pub fn router() -> Router<crate::ServerState> {
    Router::new()
        .route("/mint", post(mint_reserve))
        .route("/", get(list_reserves))
}
