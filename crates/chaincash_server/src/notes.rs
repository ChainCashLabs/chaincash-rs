use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chaincash_offchain::transactions::notes::{MintNoteRequest, SignedMintNoteResponse};
use serde_json::json;

use crate::api::ApiError;

async fn mint_note(
    State(state): State<crate::ServerState>,
    Json(body): Json<MintNoteRequest>,
) -> Result<Response, ApiError> {
    let SignedMintNoteResponse { note, transaction } = state.tx_service().mint_note(body).await?;
    let response = Json(json!({
        "txId": transaction.id().to_string(),
        "noteId": note.note_id,
    }));
    Ok(response.into_response())
}

async fn list_notes(State(state): State<crate::ServerState>) -> Result<Response, ApiError> {
    let notes = state.store.notes().notes()?;
    let response = Json(notes);
    Ok(response.into_response())
}
pub fn router() -> Router<crate::ServerState> {
    Router::new()
        .route("/", get(list_notes))
        .route("/mint", post(mint_note))
}
