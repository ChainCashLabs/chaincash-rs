use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};
use chaincash_offchain::transactions::reserves::MintReserveOpt;

async fn mint_reserve(
    State(state): State<crate::ServerState>,
    Json(body): Json<MintReserveOpt>,
) -> impl IntoResponse {
    let tx_id = state.tx_service.mint_reserve(body).unwrap();

    tx_id.to_string()
}

pub fn router() -> Router<crate::ServerState> {
    Router::new().route("/mint", post(mint_reserve))
}
