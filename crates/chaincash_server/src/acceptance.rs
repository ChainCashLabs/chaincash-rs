use std::sync::Arc;

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chaincash_services::ServerState;

use crate::api::ApiError;

async fn get_acceptance(State(state): State<Arc<ServerState>>) -> Result<Response, ApiError> {
    Ok(Json(&state.predicates).into_response())
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new().route("/", get(get_acceptance))
}
