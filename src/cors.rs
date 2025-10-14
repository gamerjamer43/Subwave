use hyper::{Response, Body, header::{HeaderValue, ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_HEADERS}};

// any response gets these headers
pub fn add_cors_headers(resp: &mut Response<Body>) {
    let headers = resp.headers_mut();

    // for right now i'm just testing. note to future retard noah: THIS IS WHY YOUR SHITS WEIRD
    headers.insert(
        ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*") // THIS SPECIFICALLY
    );

    // other actual ones we need
    headers.insert(
        ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST, OPTIONS")
    );
    headers.insert(
        ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("Content-Type")
    );
}

// handle OPTIONS preflight requests
pub fn cors_preflight() -> Response<Body> {
    let mut resp = Response::builder()
        .status(204)
        .body(Body::empty())
        .unwrap();
    add_cors_headers(&mut resp);
    resp
}