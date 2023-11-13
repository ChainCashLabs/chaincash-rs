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
    // TODO: https://github.com/ChainCashLabs/chaincash-rs/issues/13
    // remove requirement to run in separate thread
    let tx_id = std::thread::spawn(move || state.tx_service.mint_reserve(body))
        .join()
        .unwrap()?;
    let body = Json(json!({
        "txId": tx_id.to_string(),
    }));

    Ok(body.into_response())
}

pub fn router() -> Router<crate::ServerState> {
    Router::new().route("/mint", post(mint_reserve))
}
