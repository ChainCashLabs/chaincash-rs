use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use chaincash_offchain::transactions::reserves::MintReserveOpt;
use serde_json::json;

use crate::api::ApiError;

async fn mint_reserve(
    State(state): State<crate::ServerState>,
    Json(body): Json<MintReserveOpt>,
) -> Result<Response, ApiError> {
    // might need a few different types of ApiError
    // "Upstream" - to indicate if something with the node failed
    // "BadRequest" - to indicate bad user inputs
    // etc
    let tx_id = state
        .tx_service
        .mint_reserve(body)
        .map_err(|e| ApiError::TransactionBuild(e.to_string()))?;
    let body = Json(json!({
        "txId": tx_id.to_string(),
    }));

    Ok(body.into_response())
}

pub fn router() -> Router<crate::ServerState> {
    Router::new().route("/mint", post(mint_reserve))
}
