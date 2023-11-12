use axum::{
    response::{IntoResponse, Response},
    Router,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("transaction building error: {0}")]
    TransactionBuild(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        todo!()
    }
}

pub fn router() -> Router<crate::ServerState> {
    let router_v1 = Router::new().nest("/reserves", crate::reserves::router());

    Router::new().nest("/v1", router_v1)
}
