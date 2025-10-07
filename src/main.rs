// my modules
mod api;
mod cors;
mod db;
mod models;
mod scanner;
mod server;

// my cors imports cuz it's being fucky
use cors::{add_cors_headers, cors_preflight};

// stdlib
use std::{convert::Infallible, net::SocketAddr, path::Path, time::Instant};

// sqlx for sqlite shit
use sqlx::{Pool, Sqlite, // types
           SqlitePool};  // driver

// hyper, the main http server
use tokio::fs;                                           // one tokio helper
use hyper::{Body, Request, Response, Server, StatusCode, // important shit
            service::{make_service_fn, service_fn}};     // attatching said important shit (service handlers)

// tokio is like flask, needs main (cuz of async runtime)
#[tokio::main]
async fn main() {
    // setup database
    fs::create_dir_all("./data").await.expect("Failed to create ./data, where the db file is stored.");

    // if db file doesn't exist make it
    if !Path::new("./data/music.db").exists() {
        fs::File::create("./data/music.db").await.expect("Failed to create the ./data/music.db file.");
    }
    
    // create a pool for da db
    // TODO: make it create music.db if it's not there im just lazy
    let pool: Pool<Sqlite> = SqlitePool::connect("sqlite:./data/music.db")
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
            Ok::<_, Infallible>(service_fn(move |req| handle_request(req, pool.clone())))
        }
    });

    // and now like i said above
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