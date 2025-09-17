use axum::Router;
use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    handlers::AppState,
    models::*,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::images::upload_image,
        crate::handlers::images::get_image,
        crate::handlers::images::delete_image,
        crate::handlers::images::get_metadata,
        crate::handlers::auth::register,
        crate::handlers::auth::login,
        crate::handlers::auth::refresh,
        crate::handlers::admin::create_api_key,
        crate::handlers::admin::get_api_key,
        crate::handlers::admin::get_usage,
        crate::handlers::health::liveness,
        crate::handlers::health::readiness,
    ),
    components(
        schemas(
            User,
            CreateUserRequest,
            LoginRequest,
            AuthResponse,
            UserResponse,
            ApiKey,
            CreateApiKeyRequest,
            ApiKeyLimits,
            RateLimits,
            ApiKeyResponse,
            Image,
            UploadRequest,
            Visibility,
            ImageVariant,
            ImageVariants,
            UploadResponse,
            ImageMetadata,
            UsageCounter,
            UsageQuery,
            UsageResponse,
            DailyUsage,
        )
    ),
    tags(
        (name = "images", description = "Image upload and management endpoints"),
        (name = "auth", description = "Authentication endpoints"),
        (name = "admin", description = "Administrative endpoints"),
        (name = "health", description = "Health check endpoints")
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "api_key",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("x-api-key"))),
            );
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    utoipa::openapi::security::Http::new(
                        utoipa::openapi::security::HttpAuthScheme::Bearer,
                    )
                    .bearer_format("JWT"),
                ),
            );
        }
    }
}

pub fn add_docs(app: Router<AppState>) -> Router<AppState> {
    app.merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
        .route("/openapi.json", axum::routing::get(|| async {
            axum::Json(ApiDoc::openapi())
        }))
}
