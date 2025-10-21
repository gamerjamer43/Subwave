// backend related shit
use sqlx::{PgPool, Pool, Postgres};
use axum::{
    middleware::from_fn_with_state, 
    routing::{get, post},
    http::{StatusCode, Response},
    body::Body,
    Router
};

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

        // otherwise 404 that shit (will add other routes besides file serving and the api potentially)
        .fallback(|| async { status_response(StatusCode::NOT_FOUND) })
        .with_state(pool)
}