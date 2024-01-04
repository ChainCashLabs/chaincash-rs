use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use serde_json::json;
use thiserror::Error;

trait AsStatusCode {
    fn as_status_code(&self) -> StatusCode;
}

// Despite the name this doesn't represent a clientside error in the context of server/client
// it means the node client library threw an error
impl AsStatusCode for chaincash_offchain::ClientError {
    fn as_status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

impl AsStatusCode for chaincash_offchain::node::NodeError {
    fn as_status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

impl AsStatusCode for chaincash_offchain::transactions::TransactionError {
    fn as_status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

impl AsStatusCode for chaincash_offchain::Error {
    fn as_status_code(&self) -> StatusCode {
        match self {
            chaincash_offchain::Error::Client(e) => e.as_status_code(),
            chaincash_offchain::Error::Node(e) => e.as_status_code(),
            chaincash_offchain::Error::Transaction(e) => e.as_status_code(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("offchain error: {0}")]
    OffChain(#[from] chaincash_offchain::Error),

    #[error("Transaction service error")]
    TransactionServe(#[from] chaincash_services::transaction::TransactionServiceError),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status_code, msg) = match self {
            ApiError::OffChain(e) => (e.as_status_code(), e.to_string()),
            ApiError::TransactionServe(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };
        let body = Json(json!({
            "error": {
                "detail": msg,
            }
        }));

        (status_code, body).into_response()
    }
}

pub fn router() -> Router<crate::ServerState> {
    let router_v1 = Router::new()
        .nest("/reserves", crate::reserves::router())
        .nest("/notes", crate::notes::router())
        .nest("/acceptance", crate::acceptance::router());

    Router::new().nest("/v1", router_v1)
}
