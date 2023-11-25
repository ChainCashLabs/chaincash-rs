use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(paths(crate::acceptance::get_acceptance))]
pub struct OpenApiDoc;

pub fn router() -> Router<crate::ServerState> {
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", OpenApiDoc::openapi()))
    // .merge(Redoc::with_url("/redoc", OpenApiDoc::openapi()))
    // There is no need to create `RapiDoc::with_openapi` because the OpenApi is served
    // via SwaggerUi instead we only make rapidoc to point to the existing doc.
    // .merge(RapiDoc::new("/api-docs/openapi.json").path("/rapidoc"))
}
