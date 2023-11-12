use axum::Router;

pub fn router() -> Router<crate::ServerState> {
    let router_v1 = Router::new().nest("/reserves", crate::reserves::router());

    Router::new().nest("/v1", router_v1)
}
