mod mods;
mod api;

use crate::{
    mods::{scanner::scan},
    api::{
        cors::{add_cors_headers, cors_preflight}, 
        router::route
    }
};

use dotenvy::dotenv;
use std::{
    convert::Infallible, 
    env::var, net::SocketAddr, 
    cell::LazyCell, time::Instant
};

// switching this jawn over to axum
// use tower::limit::RateLimitLayer;
use axum::{
    body::{boxed, BoxBody}, Router, Server,
    extract::connect_info::ConnectInfo,
    middleware::{from_fn, Next},
    http::{Method, Request, Response},
};

// sqlx is fire use it even if u change anything
use sqlx::{PgPool, postgres::PgPoolOptions};

// address is hardcoded frn (i should prolly delegate that to env, but it would da just be port)
const ADDR: LazyCell<SocketAddr> = LazyCell::new(|| SocketAddr::from(([0, 0, 0, 0], 6000)));
const URL: LazyCell<String> = LazyCell::new(|| var("DATABASE_URL")
                                .expect("Make sure to set DATABASE_URL in .env"));

#[tokio::main]
async fn main() {
    let addr: SocketAddr = *ADDR;
    dotenv().ok();
    
    println!("\nPostgres URL: {}", URL.clone());
    println!("Listening on http://{}", addr);

    // aye aye aye i'm a basic bitch. if it ain't broke...
    let pool: PgPool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&URL).await
        .expect("Failed to connect to Postgres. Is the server running?");

    // fw this folder name if you want your shit elsewhere
    scan(&pool, "./static").await
        .expect("Failed to index songs.");

    // bind router to server, and add our request layer
    let app: Router = route(pool)
        .layer(from_fn(handle_request));

    Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await.unwrap();
}

// general sendback handler
async fn handle_request<B>(req: Request<B>, next: Next<B>) 
-> Result<Response<BoxBody>, Infallible> {
    let start: Instant = Instant::now();
    let path: String = req.uri().path().to_owned();
    let method: &Method = req.method();

    // get ip using this janky ass shit (this the only thing i dislike abt axum)
    let remote_addr = req
        .extensions().get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(addr)| *addr);

    // options preflight
    if method == Method::OPTIONS {
        return Ok(cors_preflight().map(boxed));
    }

    // route to api
    let mut response = next.run(req).await;

    // add headers and print how long that shit took
    add_cors_headers(&mut response);
    let duration_ms = start.elapsed().as_micros();

    println!(
        "{}{} {} - {}μs",
        remote_addr.map(|a| format!("{a} ")).unwrap_or_default(),
        path, response.status(), duration_ms
    );

    Ok(response)
}