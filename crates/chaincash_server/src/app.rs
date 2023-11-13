//! ChainCash payment server creation and serving.
use axum::{routing::get, Router};
use chaincash_offchain::{NodeInterface, TransactionService};
use chaincash_store::ChainCashStore;
use tokio::signal;
use tracing::info;

use crate::api;

#[derive(Clone)]
pub struct ServerState {
    pub store: ChainCashStore,
    pub node: NodeInterface,
    pub tx_service: TransactionService,
}

pub struct Server;

impl Server {
    pub fn router() -> Router<ServerState> {
        Router::new()
            .route("/healthcheck", get(|| async { "ok" }))
            .nest("/api", api::router())
    }

    /// Serves the ChainCash payment server on the given listener forever
    /// using the supplied server state.
    pub async fn serve(
        listener: std::net::TcpListener,
        state: ServerState,
    ) -> Result<(), crate::Error> {
        info!("server started on listener: {:?}", listener);

        axum::Server::from_tcp(listener)?
            .serve(Self::router().with_state(state).into_make_service())
            .with_graceful_shutdown(Self::shutdown())
            .await?;

        Ok(())
    }

    async fn shutdown() {
        let ctrl_c = async {
            signal::ctrl_c().await.expect("Cannot install handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }

        info!("shutting down server");
    }
}

#[cfg(test)]
impl ServerState {
    pub fn for_test() -> Self {
        // node shouldn't be actually used in unit tests
        let node = NodeInterface::new("hello", "127.0.0.1", "9032").unwrap();

        ServerState {
            store: ChainCashStore::open_in_memory().unwrap(),
            node: node.clone(),
            tx_service: TransactionService::new(node),
        }
    }
}

#[cfg(test)]
mod tests {
    use hyper::{Body, Request, StatusCode};
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn test_healthcheck() {
        let response = Server::router()
            .with_state(ServerState::for_test())
            .oneshot(Request::get("/healthcheck").body(Body::default()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
