//! ChainCash payment server creation and serving.
use std::sync::{Arc, RwLock};

use axum::{routing::get, Router};
use tracing::info;

use crate::kv::{HeedKvStore, KvStore};

#[derive(Clone)]
struct AppState {
    pub kv: Arc<RwLock<Box<dyn KvStore>>>,
}

fn make_app() -> Result<Router<AppState>, crate::Error> {
    let app = Router::new().route("/healthcheck", get(|| async { "ok" }));

    Ok(app)
}

/// Serves the ChainCash payment server on the given listener forever.
///
/// # Example
///
/// ```
/// # async fn run() {
/// use std::net::TcpListener;
///
/// let listener = TcpListener::bind("127.0.0.1:3000").unwrap();
///
/// chaincash_server::serve_blocking(listener).await.unwrap();
/// # }
/// ```
pub async fn serve_blocking(listener: std::net::TcpListener) -> Result<(), crate::Error> {
    let db_path = std::env::current_dir()?.join("state.mdb");

    std::fs::create_dir_all(&db_path)?;
    info!("using database path: {}", db_path.display());

    let state = AppState {
        kv: Arc::new(RwLock::new(Box::new(HeedKvStore::new(&db_path)?))),
    };

    info!("starting server");

    axum::Server::from_tcp(listener)?
        .serve(make_app()?.with_state(state).into_make_service())
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use hyper::{Body, Request, StatusCode};
    use tower::ServiceExt;

    use super::*;

    fn make_state() -> AppState {
        AppState {
            kv: Arc::new(RwLock::new(Box::new(crate::kv::InMemoryKvStore::default()))),
        }
    }

    #[tokio::test]
    async fn test_healthcheck() {
        let response = make_app()
            .unwrap()
            .with_state(make_state())
            .oneshot(Request::get("/healthcheck").body(Body::default()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
