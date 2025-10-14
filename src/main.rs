// my modules
mod api;
mod cors;
mod db;
mod login;
mod models;
mod scanner;
mod server;

// my cors imports cuz it's being fucky
use cors::{add_cors_headers, cors_preflight};

// stdlib
use std::{convert::Infallible, net::SocketAddr, fs::File, time::Instant};

// sqlx for sqlite shit
use sqlx::{Pool, Sqlite, // types
           SqlitePool};  // driver

// hyper, the main http server
use tokio::fs;                                           // one tokio helper
use hyper::{Body, Request, Response, Server, StatusCode, // important shit
            service::{make_service_fn, service_fn}};     // attatching said important shit (service handlers)

// put this inside main imports if it works
use hyper::server::conn::AddrStream;

// tokio is like flask, needs main (cuz of async runtime)
#[tokio::main]
async fn main() {
    // ensure the directory exists
    fs::create_dir_all("./data")
        .await
        .expect("Failed to create ./data, where the db file is stored.");

    // create db if it doesn't exist
    let db_path = "./data/music.db";
    if !std::path::Path::new(db_path).exists() {
        File::create(db_path)
            .expect("Failed to create database file");
        println!("Created database file at {}", db_path);
    }

    // open pool and connect (SQLite creates the file if missing)
    let pool: Pool<Sqlite> = SqlitePool::connect("sqlite://./data/music.db")
        .await
        .expect("Failed to connect to database");

    // initialize database tables
    db::init(&pool).await.expect("DB initialization failed");

    // scan and index music files
    scanner::scan(&pool, "./static").await.expect("Failed to scan music files");

    // bind server
    let addr = SocketAddr::from(([0, 0, 0, 0], 6000));
    println!("Listening on http://{}", addr);

    let service = make_service_fn(move |conn: &AddrStream| {
        let pool = pool.clone();
        let remote_addr = conn.remote_addr();
        println!("New connection from {}", remote_addr);

        async move {
            Ok::<_, Infallible>(service_fn(move |req| handle_request(req, pool.clone())))
        }
    });

    Server::bind(&addr).serve(service).await.unwrap();
}

// handle individual requests
async fn handle_request(req: Request<Body>, pool: SqlitePool) -> Result<Response<Body>, Infallible> {
    // resolve path
    let start = Instant::now();
    let path = req.uri().path();
    
    // handle OPTIONS preflight requests
    if req.method() == hyper::Method::OPTIONS {
        return Ok(cors_preflight());
    }
    
    // api searches go thru /api/
    let mut resp = if path.starts_with("/api/") {
        match api::handle(req, pool).await {
            Ok(r) => r,
            Err(e) => error(e),
        }
    }

    // static files go thru /file/
    else if path.starts_with("/file/") {
        match server::serve(req).await {
            Ok(r) => r,
            Err(e) => error(e),
        }
    }

    // otherwise 404 that shit if we don't have one
    else {
        Response::builder()
            .status(404)
            .body(Body::from("Not Found"))
            .unwrap()
    };

    // add headers and print how long that shit took
    add_cors_headers(&mut resp);
    println!("{} - {:.2}ms", resp.status(), start.elapsed().as_secs_f64() * 1000.0);
    Ok(resp)
}

// simple error handler
fn error(status: StatusCode) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(Body::from(status.canonical_reason().unwrap_or("Error")))
        .unwrap()
}