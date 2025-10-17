// backend related shit
use hyper::{Body, Request, Response, StatusCode, Method, header};
use tokio::fs;
use tokio_util::io::ReaderStream;
use sqlx::{SqlitePool, Row};

// escaping searches
use percent_encoding::percent_decode_str;

// all necessary models
use crate::models::{Song, Album, AuthRequest};

// login helpers
use crate::login::{signup, login, verify};

// basic handler
pub async fn handle(req: Request<Body>, pool: SqlitePool) -> Result<Response<Body>, StatusCode> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    // post is legit only used for auth. (may be used for uploading, but...) everything else is a get
    if method == Method::POST && (path == "/api/signup" || path == "/api/login") {
        let body = hyper::body::to_bytes(req.into_body()).await.map_err(|_| StatusCode::BAD_REQUEST)?;
        let auth: AuthRequest = serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;
        return match path.as_str() {
            "/api/signup" => signup(pool, auth).await,
            "/api/login"  => login(pool, auth).await,
            _ => unreachable!(),
        };
    }

    // auth guard for non-auth routes
    if let Some(resp) = auth(&pool, &req).await {return Ok(resp);}
    if method != Method::GET {return Err(StatusCode::METHOD_NOT_ALLOWED);}

    // router part 2 electric boogaloo, only for the actual api methods
    match path.as_str() {
        p if p.starts_with("/api/search") => search(req, pool.clone()).await,
        p if p.starts_with("/api/cover") => cover(p, pool.clone()).await,
        p if p.starts_with("/api/album") => album(p, pool.clone()).await,
        p if p == "/api/test" => test(pool.clone(), req).await,
        p if p.starts_with("/file/") => serve(p, req).await,
        _ => Err(StatusCode::NOT_FOUND),
    }
}

// if no auth, get fucked
async fn auth(pool: &SqlitePool, req: &Request<Body>) -> Option<Response<Body>> {
    if let Err(status) = verify(pool, req).await {
        return Some(
            Response::builder()
                .status(status)
                .body(Body::from("unauthorized"))
                .unwrap(),
        );
    }
    None
}

// basic token status check
pub async fn test(pool: SqlitePool, req: Request<Body>) -> Result<Response<Body>, StatusCode> {
    if let Some(resp) = auth(&pool, &req).await {return Ok(resp);}

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"status":"ok"}"#))
        .map_err(|_| StatusCode::UNAUTHORIZED)
}

// file service
pub async fn serve(path: &str, _req: Request<Body>) -> Result<Response<Body>, StatusCode> {
    // strip /file/ prefix from the path, index.html if none (which i havent added)
    // TODO: add escaping so we can't jack the db. idk if this is vulnerable or not but we'll look later
    let filepath = percent_decode_str(path.trim_start_matches("/file/"))
                      .decode_utf8_lossy().to_string();
    let filepath = if filepath.is_empty() {"index.html".to_string()} else {filepath};

    // prevent directory traversal
    if filepath.contains("..") {
        return Err(StatusCode::FORBIDDEN);
    }

    // build the full file path and try to open
    let filepath = format!("./static/{}", filepath);
    let file = fs::File::open(&filepath).await
              .map_err(|_| StatusCode::NOT_FOUND)?;
    
    // get dat metadata
    let metadata = file.metadata().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // wrap file in a streaming body
    let body = Body::wrap_stream(ReaderStream::new(file));
    
    // determine content type based on file extension
    let content_type = match filepath.rsplit('.').next() {
        // audio content
        Some("ogg") => "audio/ogg",
        Some("mp3") => "audio/mpeg",
        Some("flac") => "audio/flac",
        Some("wav") => "audio/wav",

        // site content
        Some("html") => "text/html",
        Some("css") => "text/css",
        Some("js") => "application/javascript",

        // otherwise a stream
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

// song search
async fn search(req: Request<Body>, pool: SqlitePool) -> Result<Response<Body>, StatusCode> {
    // the shit we need to escape
    let raw_query = req.uri().query().unwrap_or("");

    // build a query from this
    // TODO: add escaping so we can't jack the db
    let search_term = raw_query
        .split('&')
        .find_map(|kv| kv.strip_prefix("q="))
        .unwrap_or("");  

    // decode any percent-encoded characters, trim quotes (we need a second borrow)
    let search_term = percent_decode_str(search_term).decode_utf8_lossy();
    let search_term = search_term.trim_matches('"').trim();

    // build the pattern and search
    let pattern = &format!("%{}%", search_term);
    let sql = include_str!("../queries/searchsong.sql");

    // run the query dynamically
    let rows = sqlx::query(sql)
        .bind(pattern)
        .bind(pattern)
        .bind(pattern)
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // returned songs go here
    let songs: Vec<Song> = rows
        .iter()
        .map(|row| {
            Song {
                id: row.try_get::<i64, _>("id").unwrap_or(0) as u16,
                name: row.try_get::<String, _>("name").unwrap_or_default(),
                artist: row.try_get::<String, _>("artist").unwrap_or_default(),
                album: row.try_get::<String, _>("album").unwrap_or_default(),
                cover: None, // don't send cover in list view
                duration: row.try_get::<i64, _>("duration").unwrap_or(0) as u16,
                filename: row.try_get::<String, _>("filename").unwrap_or_default(),
            }
        })
        .collect();
    
    // serde serializes this shit
    let json = serde_json::to_string(&songs)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // send back an ok!!!
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(Body::from(json))
        .unwrap())
}

// cover search
async fn cover(path: &str, pool: SqlitePool) -> Result<Response<Body>, StatusCode> {
    // parse album id
    let id: i64 = path.trim_start_matches("/api/cover/")
        .parse()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    // find cover with that id
    let cover: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT a.cover FROM songs s JOIN albums a ON s.album_id = a.id WHERE s.id = ?"
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;

    // return that hoe (or error if magic has happened. check if you deleted the cover metadata)
    let data = cover.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Response::builder()
        .body(Body::from(data))
        .unwrap())
}

// album search
async fn album(path: &str, pool: SqlitePool) -> Result<Response<Body>, StatusCode> {
    // album id (yoinked from above)
    let album_id: u16 = path.trim_start_matches("/api/album/")
        .parse()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // album metadata
    let row = sqlx::query("SELECT id, name, artist, cover, runtime, songcount FROM albums WHERE id = ?")
        .bind(album_id)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // fetch songs
    let songs: Vec<Song> = sqlx::query_as("SELECT s.id, s.name, s.duration, s.filename, a.name as album, a.artist as artist FROM songs s JOIN albums a ON s.album_id = a.id WHERE a.id = ? ORDER BY s.track_number ASC")
        .bind(album_id)
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // build into one big fat album
    let resp = Album {
        id: row.get("id"),
        name: row.get("name"),
        artist: row.get("artist"),
        runtime: row.get("runtime"),
        songcount: row.get("songcount"),
        songs,
    };

    // serialize and send
    let json = serde_json::to_string(&resp).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Response::builder().body(Body::from(json)).unwrap())
}
