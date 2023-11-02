//! ChainCash payment server creation and serving.
use axum::{routing::get, Router};
use chaincash_store::ChainCashStore;
use tracing::info;

#[derive(Clone)]
struct AppState {
    pub store: ChainCashStore,
}

fn make_app() -> Result<Router<AppState>, crate::Error> {
    let app = Router::new().route("/healthcheck", get(|| async { "ok" }));

    Ok(app)
}

/// Serves the ChainCash payment server on the given listener forever
/// using the supplied chaincash store.
pub async fn serve_blocking(
    listener: std::net::TcpListener,
    store: ChainCashStore,
) -> Result<(), crate::Error> {
    let state = AppState { store };

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
            store: ChainCashStore::open_in_memory().unwrap(),
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
