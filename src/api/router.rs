use std::sync::{Arc, LazyLock};

use axum::{
    body::Body,
    http::{Response, StatusCode},
    middleware::from_fn_with_state,
    routing::{get, post},
    Router,
};
use governor::middleware::NoOpMiddleware;
use sqlx::{PgPool, Pool, Postgres};
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
};

// my routes
use crate::api::{
    cors::add_cors_headers,
    endpoints::{album, cover, require_auth, search, serve, test},
    login::{login, signup},
};

// this is what does the rate limiting
pub static GOVERNOR_LAYER: LazyLock<GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware, Body>> =
    LazyLock::new(|| {
        let config = Arc::new(
            GovernorConfigBuilder::default()
                .per_second(1)
                .burst_size(30)
                .key_extractor(SmartIpKeyExtractor)
                .finish()
                .expect("valid governor config"),
        );
        GovernorLayer::new(config)
    });

/// helper response
pub fn status_response(status: StatusCode) -> Response<Body> {
    let mut resp = Response::builder()
        .status(status)
        .body(Body::from(status.canonical_reason().unwrap_or("Error")))
        .unwrap();

    add_cors_headers(&mut resp);
    resp
}

/// router definition
pub fn route(pool: PgPool) -> Router {
    let gated: Router<Pool<Postgres>> = Router::new()
        .route("/api/test", get(test))
        .route("/api/search", get(search))
        .route("/api/cover/{rest}", get(cover))
        .route("/api/album/{rest}", get(album))
        .route("/file/{*path}", get(serve))
        .route_layer(from_fn_with_state(pool.clone(), require_auth));

    Router::new()
        .route("/api/signup", post(signup))
        .route("/api/login", post(login))
        .merge(gated)
        .layer(GOVERNOR_LAYER.clone())
        .fallback(|| async { status_response(StatusCode::NOT_FOUND) })
        .with_state(pool)
}