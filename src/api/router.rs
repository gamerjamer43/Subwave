use std::sync::Arc;

// backend related shit
use sqlx::{PgPool, Pool, Postgres};
use axum::{
    middleware::from_fn_with_state, 
    routing::{get, post},
    http::{StatusCode, Response},
    body::Body,
    Router
};
use tower_governor::{governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer};

// my routes
use crate::api::{
    login::{login, signup},
    cors::add_cors_headers,
    endpoints::{
        album, cover, search, 
        serve, test, 
        require_auth
    }
};

pub fn status_response(status: StatusCode) -> Response<Body> {
    let mut resp = Response::builder()
        .status(status)
        .body(Body::from(status
            .canonical_reason()
            .unwrap_or("Error")
        )).unwrap();

    add_cors_headers(&mut resp);
    resp
}

// routes urls to proper path (axum!!!! yay!!!)
pub fn route(pool: PgPool) -> Router {
    // default to 20 per second (note this builds EVERY TIME something is routed... i need to figure this shit out.)
    let governor_config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(1)
            .burst_size(20)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("valid governor config"),
    );

    // build a governor layer
    let governor_layer = GovernorLayer::<SmartIpKeyExtractor, _, Body>::new(governor_config);

    let gated: Router<Pool<Postgres>> = Router::new()
        .route("/api/test", get(test))
        .route("/api/search", get(search))
        .route("/api/cover/*rest", get(cover))
        .route("/api/album/*rest", get(album))
        .route("/file/*path", get(serve))
        .route_layer(from_fn_with_state(pool.clone(), require_auth));

    Router::new()
        // post is legit only used for auth. (may be used for uploading, but...) everything else is a get
        .route("/api/signup", post(signup))
        .route("/api/login", post(login))
        .merge(gated)
        .layer(governor_layer)

        // otherwise 404 that shit (will add other routes besides file serving and the api potentially)
        .fallback(|| async { status_response(StatusCode::NOT_FOUND) })
        .with_state(pool)
}
