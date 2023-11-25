use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};

use crate::api::ApiError;

#[utoipa::path(get, path = "/api/v1/acceptance", responses((status = StatusCode::OK)))]
async fn get_acceptance(State(state): State<crate::ServerState>) -> Result<Response, ApiError> {
    Ok(Json(state.predicates).into_response())
}

pub fn router() -> Router<crate::ServerState> {
    Router::new().route("/", get(get_acceptance))
}
