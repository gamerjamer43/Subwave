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
use std::{convert::Infallible, net::SocketAddr, time::Instant};

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
    fs::create_dir_all("./data").await.expect("Failed to create data directory");
    
    // create a pool for da db
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
        // copy pool
        let pool = pool.clone();

        // make service enclosure
        async move {
            // further service enclosure
            Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                // make a copy of the pool
                let pool = pool.clone();

                // wrap with an async for... yk
                async move {
                    // time that hoe
                    let start = Instant::now();
                    
                    // resolve path
                    let path = req.uri().path();
                    
                    // handle OPTIONS preflight requests
                    if req.method() == hyper::Method::OPTIONS {
                        return Ok::<_, Infallible>(cors_preflight());
                    }
                    
                    let mut resp = if path.starts_with("/api/") {
                        println!("DEBUG: Routing to API");
                        match api::handle(req, pool).await {
                            Ok(r) => r,
                            Err(e) => error(e),
                        }
                    }

                    // thrashed for 20 min and i am late to discrete. it works tho.
                    // i can't believe i'm so fuckin stupid.
                    else if path.starts_with("/file/") {
                        match server::serve(req).await {
                            Ok(r) => r,
                            Err(e) => error(e),
                        }
                    }

                    // otherwise nope
                    else {
                        Response::builder()
                            .status(404)
                            .body(Body::from("Not Found"))
                            .unwrap()
                    };

                    // add CORS headers to response
                    add_cors_headers(&mut resp);

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