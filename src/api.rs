// backend related shit
use hyper::{Body, Request, Response, StatusCode, Method, header};
use tokio::fs;
use tokio_util::io::ReaderStream;
use sqlx::{PgPool, Row};

// escaping searches
use percent_encoding::{percent_decode_str};

// all necessary models
use crate::models::{Song, Album, AuthRequest};

// login helpers
use crate::cors::{add_cors_headers};
use crate::login::{signup, login, verify};

// song search helper
const SEARCH: String = include_str!("../queries/searchsong.sql");

// basic handler
pub async fn handle(req: Request<Body>, pool: PgPool) -> Result<Response<Body>, StatusCode> {
    // i'm allocating needlessly here, dubbin that in the next patch
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
async fn auth(pool: &PgPool, req: &Request<Body>) -> Option<Response<Body>> {
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
pub async fn test(pool: PgPool, req: Request<Body>) -> Result<Response<Body>, StatusCode> {
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
    
    // accept ranges is for streaming btw (just for y'all who don't know)
    let mut response = Response::builder()
        .header("Content-Type", content_type)
        .header("Content-Length", metadata.len())
        .header("Accept-Ranges", "bytes")
        .body(body)
        .unwrap();

    add_cors_headers(&mut response);
    Ok(response)
}

// song search
async fn search(req: Request<Body>, pool: PgPool) -> Result<Response<Body>, StatusCode> {
    // get raw query
    let raw = req.uri().query().unwrap_or("");
    let search = raw
        .split('&')
        .find_map(|kv| kv.strip_prefix("q="))
        .unwrap_or("");  

    // build search pattern
    let search = percent_decode_str(search).decode_utf8_lossy();
    let search_term = search.trim_matches('"').trim();
    let pattern = format!("%{}%", search_term);

    // query for songs
    let query = sqlx::query(&SEARCH)
        .bind(pattern.as_str())
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // list of matched songs
    let songs: Vec<Song> = query
        .iter()
        .map(|row| {
            Song {
                id: row.try_get::<i32, _>("id").unwrap_or(0),
                name: row.try_get::<String, _>("name").unwrap_or_default(),
                artist: row.try_get::<String, _>("artist").unwrap_or_default(),
                album: row.try_get::<String, _>("album").unwrap_or_default(),
                duration: row.try_get::<i32, _>("duration").unwrap_or(0),
                filename: row.try_get::<String, _>("filename").unwrap_or_default(),
                cover: None,
            }
        })
        .collect();
    
    // serialize and shoot er back
    let json = serde_json::to_string(&songs).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(Body::from(json))
        .unwrap())
}

// cover search
async fn cover(path: &str, pool: PgPool) -> Result<Response<Body>, StatusCode> {
    // parse album id
    let id: i64 = path.trim_start_matches("/api/cover/").parse()
                      .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    // find cover with that id
    let cover: Option<Vec<u8>> = sqlx::query_scalar("SELECT a.cover FROM songs s JOIN albums a ON s.album_id = a.id WHERE s.id = $1")
        .bind(id)
        .fetch_one(&pool).await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // return that hoe (or error if magic has happened. you prolly deleted a cover)
    let data = cover.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Response::builder()
        .body(Body::from(data))
        .unwrap())
}

// album search
async fn album(path: &str, pool: PgPool) -> Result<Response<Body>, StatusCode> {
    // album id (yoinked from above)
    let album_id: i32 = path.trim_start_matches("/api/album/")
        .parse()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // album metadata
    let row = sqlx::query("SELECT id, name, artist, cover, runtime, songcount FROM albums WHERE id = $1")
        .bind(album_id)
        .fetch_one(&pool).await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // fetch songs
    let songs: Vec<Song> = sqlx::query_as::<_, Song>(
        "SELECT s.id, s.name, a.artist, a.name AS album, a.cover, s.duration, s.filename \
         FROM songs s \
         JOIN albums a ON s.album_id = a.id \
         WHERE a.id = $1 \
         ORDER BY s.track_number ASC"
    )
        .bind(album_id)
        .fetch_all(&pool).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // build into one big fat album
    let resp = Album {
        id: row.get("id"),
        name: row.get("name"),
        artist: row.get("artist"),
        runtime: row.get::<i32, _>("runtime"),
        songcount: row.get::<i32, _>("songcount"),
        songs,
    };

    // serialize and send
    let json = serde_json::to_string(&resp).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Response::builder().body(Body::from(json)).unwrap())
}