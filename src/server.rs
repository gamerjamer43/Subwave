use hyper::{Body, Request, Response, StatusCode, Method, header};
use tokio::fs;
use tokio_util::io::ReaderStream;

pub async fn serve(req: Request<Body>) -> Result<Response<Body>, StatusCode> {
    if req.method() != Method::GET {
        return Err(StatusCode::METHOD_NOT_ALLOWED);
    }

    let path = req.uri().path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    
    if path.contains("..") {
        return Err(StatusCode::FORBIDDEN);
    }

    let file_path = format!("./static/{}", path);
    let file = fs::File::open(&file_path).await
              .map_err(|_| StatusCode::NOT_FOUND)?;
    
    let metadata = file.metadata().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let body = Body::wrap_stream(ReaderStream::new(file));
    
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