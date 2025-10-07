// my modules
mod models;
mod db;
mod scanner;
mod api;
mod server;

// stdlib
use std::{convert::Infallible, net::SocketAddr, time::Instant};

// sqlx for sqlite shit
use sqlx::SqlitePool;

// hyper, the main http server
use hyper::{Body, Request, Response, Server, StatusCode, service::{make_service_fn, service_fn}};

// tokio file helpers 
use tokio::fs;

// tokio is like flask, needs main (cuz of async runtime)
#[tokio::main]
async fn main() {
    // setup database
    fs::create_dir_all("./data").await.expect("Failed to create data directory");
    
    // create a pool for da db
    let pool = SqlitePool::connect("sqlite:./data/music.db")
                             .await.expect("Failed to connect to database");

    // and send it to the initializer
    db::init(&pool).await.unwrap();
    
    // scan and index audio files
    scanner::scan(&pool, "./static").await.unwrap();
    
    // build a socket address, announce that we're listening on that
    let addr = SocketAddr::from(([0, 0, 0, 0], 5000));
    println!("Listening on http://{}", addr);

    // build a service from the handler (which we need to bind to)
    let service = make_service_fn(move |_| {
        let pool = pool.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                let pool = pool.clone();
                async move {
                    let start = Instant::now();
                    
                    // route api requests properly
                    let resp = if req.uri().path().starts_with("/api/") {
                        match api::handle(req, pool).await {
                            Ok(r) => r,
                            Err(e) => error(e),
                        }
                    } 
                    
                    // otherwise try and serve normally
                    // TODO: change this to /file
                    else {
                        match server::serve(req).await {
                            Ok(r) => r,
                            Err(e) => error(e),
                        }
                    };
                    
                    // print how long that shit took
                    println!("{} - {:.2}ms", resp.status(), start.elapsed().as_secs_f64() * 1000.0);
                    Ok::<_, Infallible>(resp)
                }
            }))
        }
    });

    // and now like i said above
    Server::bind(&addr).serve(service).await.unwrap();
}

// simple error handler
fn error(status: StatusCode) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(Body::from(status.canonical_reason().unwrap_or("Error")))
        .unwrap()
}