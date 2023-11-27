use axum::body::Body;
use axum::response::{IntoResponse, Response};
use axum::Router;
use hyper::StatusCode;
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
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status_code, msg) = match self {
            ApiError::OffChain(e) => (e.as_status_code(), e.to_string()),
        };
        let body = Body::from(json!({
            "error": {
                "detail": msg,
            }
        }).to_string());


        Response::builder()
            .status(status_code.as_u16())
            .header("Content-Type", "application/json")
            .body(body)
            .unwrap()
    }
}

pub fn router() -> Router<crate::ServerState> {
    let router_v1 = Router::new()
        .nest("/reserves", crate::reserves::router())
        .nest("/acceptance", crate::acceptance::router());

    Router::new().nest("/v1", router_v1)
}
