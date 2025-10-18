mod api;
mod cors;
mod db;
mod login;
mod models;
mod scanner;

use crate::cors::{add_cors_headers, cors_preflight};
use std::{convert::Infallible, net::SocketAddr, 
          time::Instant, env::var, cell::LazyCell};

// sqlx for postgres pooling
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

// hyper, the main http server
use hyper::{Body, Request, Response, Server, server::conn::{AddrStream},
            service::{make_service_fn, service_fn}};

// dot env
use dotenvy::dotenv;

// address is hardcoded frn (i should prolly delegate that to env, but it would da just be port)
const DBURL: LazyCell<String> = LazyCell::new(|| {
    var("DATABASE_URL").expect("Set DATABASE_URL in .env")
});

const ADDR: LazyCell<SocketAddr> = LazyCell::new(|| {
    SocketAddr::from(([0, 0, 0, 0], 6000))
});

#[tokio::main]
async fn main() {
    dotenv().ok();
    println!("Connected to: {}", DBURL.clone());

    // aye aye aye i'm a basic bitch. if it ain't broke...
    let pool: PgPool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&DBURL).await
        .expect("Failed to connect to Postgres. Is the server running?");

    // db initializers
    db::init(&pool).await.expect("DB initialization failed");
    scanner::scan(&pool, "./static").await.expect("Failed to index songs.");

    println!("Listening on http://{}", ADDR.clone());
    let service = make_service_fn(move |conn: &AddrStream| {
        // clone BEFORE moving (rust moment)
        let pool = pool.clone();

        // if ur using a tunnel this will be your local ip
        let remote_addr = conn.remote_addr();
        println!("New connection from {}", remote_addr);

        async move { 
            Ok::<_, Infallible>(service_fn(move |req| handle_request(req, pool.clone())))
        }
    });

    // hyper makes this shit so fun
    Server::bind(&ADDR).serve(service).await.unwrap();
}

// general sendback handler
async fn handle_request(req: Request<Body>, pool: PgPool) -> Result<Response<Body>, Infallible> {
    let start = Instant::now();
    let path = req.uri().path();
    
    // options preflight
    if req.method() == hyper::Method::OPTIONS {
        return Ok(cors_preflight());
    }
    
    // route to api
    let mut response = if path.starts_with("/api/") || path.starts_with("/file/") {
        match api::handle(req, pool).await {
            Ok(resp) => resp,
            Err(error) => Response::builder()
                                    .status(error)
                                    .body(Body::from(error.canonical_reason().unwrap_or("Error")))
                                    .unwrap(),
        }
    }

    // otherwise 404 that shit (will add other routes besides file serving and the api potentially)
    else {
        Response::builder()
            .status(404)
            .body(Body::from("Not Found"))
            .unwrap()
    };

    // add headers and print how long that shit took
    add_cors_headers(&mut response);
    println!("{} - {:.2}ms", response.status(), start.elapsed().as_secs_f64() * 1000.0);
    Ok(response)
}