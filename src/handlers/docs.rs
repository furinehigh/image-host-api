use axum::{routing::get, Router};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use crate::handlers::AppState;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::health::health_check,
        crate::handlers::upload::upload_image,
        crate::handlers::images::get_image,
        crate::handlers::images::transform_image,
        crate::handlers::images::delete_image,
        crate::handlers::user::get_quota,
    ),
    components(
        schemas(
            crate::models::ImageResponse,
            crate::models::ImageTransformParams,
            crate::models::UserQuotaResponse,
        )
    ),
    tags(
        (name = "images", description = "Image management endpoints"),
        (name = "user", description = "User management endpoints"),
        (name = "health", description = "Health check endpoints")
    ),
    info(
        title = "Image Hosting API",
        version = "1.0.0",
        description = "A high-performance image hosting server with transformation capabilities",
        contact(
            name = "API Support",
            email = "support@example.com"
        )
    )
)]
pub struct ApiDoc;

pub fn create_docs_router() -> Router<AppState> {
    Router::new()
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
}
