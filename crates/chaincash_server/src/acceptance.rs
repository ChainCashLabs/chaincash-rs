use axum::{
    extract::State,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};

use crate::api::ApiError;

async fn get_acceptance(State(state): State<crate::ServerState>) -> Result<Response, ApiError> {
    Ok(Json(state.predicates).into_response())
}

pub fn router() -> Router<crate::ServerState> {
    Router::new().route("/", get(get_acceptance))
}
