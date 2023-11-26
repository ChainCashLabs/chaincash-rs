use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use chaincash_offchain::transactions::reserves::MintReserveRequest;
use serde_json::json;

use crate::api::ApiError;

async fn mint_reserve(
    State(state): State<crate::ServerState>,
    Json(body): Json<MintReserveRequest>,
) -> Result<Response, ApiError> {
    let tx_id = state.tx_service().mint_reserve(body).await?;
    // store the reserver now or when it hits the chain? we probably need to track the box?
    let response = Json(json!({
        "txId": tx_id
    }));
    Ok(response.into_response())
}

pub fn router() -> Router<crate::ServerState> {
    Router::new().route("/mint", post(mint_reserve))
}
