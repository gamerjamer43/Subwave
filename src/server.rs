use hyper::{Body, Request, Response, StatusCode, Method, header};
use tokio::fs;
use tokio_util::io::ReaderStream;

pub async fn serve(req: Request<Body>) -> Result<Response<Body>, StatusCode> {
    // only allow GET requests
    if req.method() != Method::GET {
        return Err(StatusCode::METHOD_NOT_ALLOWED);
    }

    // strip /file/ prefix from the path, index.html if none (which i havent added)
    // TODO: add escaping so we can't jack the db
    let path = req.uri().path().trim_start_matches("/file/");
    let path = if path.is_empty() { "index.html" } else { path };

    // prevent directory traversal
    if path.contains("..") {
        return Err(StatusCode::FORBIDDEN);
    }

    // build the full file path and try to open
    let file_path = format!("./static/{}", path);
    let file = fs::File::open(&file_path).await
              .map_err(|_| StatusCode::NOT_FOUND)?;
    
    // get dat metadata
    let metadata = file.metadata().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // wrap file in a streaming body
    let body = Body::wrap_stream(ReaderStream::new(file));
    
    // determine content type based on file extension
    let content_type = match path.rsplit('.').next() {
        Some("ogg") => "audio/ogg",
        Some("mp3") => "audio/mpeg",
        Some("flac") => "audio/flac",
        Some("wav") => "audio/wav",
        Some("html") => "text/html",
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        _ => "application/octet-stream",
    };
    
    // build and return the response
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, metadata.len())
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::ACCESS_CONTROL_ALLOW_METHODS, "GET, OPTIONS")
        .header(header::ACCESS_CONTROL_ALLOW_HEADERS, "Range, Content-Type")
        .header(header::ACCEPT_RANGES, "bytes")
        .body(body)
        .unwrap())
}