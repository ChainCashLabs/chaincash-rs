//! ChainCash payment server creation and serving.
use axum::{routing::get, Router};
use chaincash_predicate::predicates::Predicate;
use chaincash_services::transaction::TransactionService;
use chaincash_store::ChainCashStore;
use ergo_client::node::NodeClient;
use tracing::info;

use crate::api;

#[derive(Clone)]
pub struct ServerState {
    pub store: ChainCashStore,
    pub node: NodeClient,
    pub predicates: Vec<Predicate>,
}

impl ServerState {
    pub fn tx_service(&self) -> TransactionService {
        TransactionService::new(&self.node, &self.store)
    }
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

        axum::serve::serve(
            listener.try_into()?,
            Self::router().with_state(state).into_make_service(),
        )
        .await?;

        Ok(())
    }
}

#[cfg(test)]
impl ServerState {
    pub fn for_test() -> Self {
        // node shouldn't be actually used in unit tests
        let node = NodeClient::from_url_str(
            "http://127.0.0.1:9052",
            "hello".to_string(),
            std::time::Duration::from_secs(5),
        )
        .unwrap();

        ServerState {
            store: ChainCashStore::open_in_memory().unwrap(),
            node,
            predicates: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
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
