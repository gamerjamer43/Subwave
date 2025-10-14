// some basic unit testing bullshit
use flacend as app;
use sqlx::SqlitePool;
use hyper::{Body, Request, StatusCode};
use serde_json;

// legit just makes a new pool what did u expect
async fn make_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.expect("connect db");
    app::db::init(&pool).await.expect("init db");
    pool
}

// signup and login testing
#[tokio::test]
async fn signup_and_login_flow() {
    let pool = make_pool().await;

    // signup
let signup_req: app::login::AuthRequest = serde_json::from_str(r#"{"username":"edgeuser","password":"hunter2"}"#).unwrap();
    let signup_res = app::login::signup(pool.clone(), signup_req).await.expect("signup should succeed");
    assert_eq!(signup_res.status(), StatusCode::CREATED);

    // login with correct password
let login_req: app::login::AuthRequest = serde_json::from_str(r#"{"username":"edgeuser","password":"hunter2"}"#).unwrap();
    let login_res = app::login::login(pool.clone(), login_req).await.expect("login should succeed");
    assert_eq!(login_res.status(), StatusCode::OK);

    // login with wrong password -> Err(UNAUTHORIZED)
let bad_req: app::login::AuthRequest = serde_json::from_str(r#"{"username":"edgeuser","password":"badpass"}"#).unwrap();
    let bad = app::login::login(pool.clone(), bad_req).await;
    assert!(matches!(bad, Err(StatusCode::UNAUTHORIZED)));
}

// cover endpoint tests
#[tokio::test]
async fn api_cover_bad_id_returns_bad_request() {
    let pool = make_pool().await;

    let req = Request::builder()
        .uri("/api/cover/not-a-number")
        .body(Body::empty())
        .unwrap();

    let res = app::api::handle(req, pool).await;
    assert!(matches!(res, Err(StatusCode::BAD_REQUEST)));
}

// file endpoint tests
#[tokio::test]
async fn server_forbids_directory_traversal() {
    // percent-encoded traversal and raw traversal should both be rejected
    let req = Request::builder()
        .uri("/file/../secret.txt")
        .body(Body::empty())
        .unwrap();

    let res = app::server::serve(req).await;
    assert!(matches!(res, Err(StatusCode::FORBIDDEN)));

    let req2 = Request::builder()
        .uri("/file/%2E%2E%2Fsecret.txt")
        .body(Body::empty())
        .unwrap();

    let res2 = app::server::serve(req2).await;
    assert!(matches!(res2, Err(StatusCode::FORBIDDEN)));
}

// long ass queries
#[tokio::test]
async fn api_search_handles_very_long_queries() {
    let pool = make_pool().await;

    // Create a very long query string to exercise edge case handling
    let long = std::iter::repeat('a').take(20_000).collect::<String>();
    let uri = format!("/api/search?q={}", long);
    let req = Request::builder().uri(uri).body(Body::empty()).unwrap();

    // We only assert that the handler finishes without panicking; it may return 200 or 500
    let _res = app::api::handle(req, pool).await;
}