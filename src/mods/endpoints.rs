// backend related shit
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use sqlx::{PgPool, Row};
use tokio::fs;
use tokio_util::io::ReaderStream;

// escaping searches
use percent_encoding::percent_decode_str;
use std::collections::HashMap;

// all necessary models
use crate::mods::models::{Album, Song};

// login helpers
use crate::mods::cors::add_cors_headers;
use crate::mods::login::verify;
use crate::mods::router::{status_response};

// song search helper
const SEARCHSONG: &str = include_str!("queries/searchsong.sql");
const SEARCHALBUM: &str = include_str!("queries/searchalbum.sql");

// if no auth, get fucked
pub async fn auth(
    pool: &PgPool,
    req: &Request<Body>
) -> Option<Response<Body>> {
    if let Err(status) = verify(pool, req.headers()).await {
        return Some(
            Response::builder()
                .status(status)
                .body(Body::from("unauthorized"))
                .unwrap(),
        );
    }
    None
}

// auth middleware
pub async fn require_auth<B>(
    State(pool): State<PgPool>,
    req: Request<B>,
    next: Next<B>,
) -> Response
where B: Send + 'static, {
    match verify(&pool, req.headers()).await {
        Ok(_) => next.run(req).await,
        Err(status) => (status, "unauthorized").into_response(),
    }
}

// basic token status check
pub async fn test(
    State(pool): State<PgPool>, 
    req: Request<Body>
) -> Response<Body> {
    if let Some(resp) = auth(&pool, &req).await {
        return resp;
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"status":"ok"}"#))
        .unwrap()
}

// file service
pub async fn serve(Path(path): Path<String>) -> Response<Body> {
    // strip /file/ prefix from the path, index.html if none (which i havent added)
    // TODO: add escaping so we can't jack the db. idk if this is vulnerable or not but we'll look later
    let raw = percent_decode_str(&path).decode_utf8_lossy().to_string();
    let filepath = if raw.is_empty() { "index.html".to_string() } else { raw };

    // prevent directory traversal
    // TODO: make this more robust this shit is not guarding
    if filepath.contains("..") {
        return status_response(StatusCode::FORBIDDEN);
    }

    // build the full file path and try to open
    let filepath = format!("./static/{}", filepath);
    let file = match fs::File::open(&filepath).await {
        Ok(f) => f,
        Err(_) => return status_response(StatusCode::NOT_FOUND),
    };

    // get dat metadata
    let metadata = match file.metadata().await {
        Ok(m) => m,
        Err(_) => return status_response(StatusCode::INTERNAL_SERVER_ERROR),
    };

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

    // accept ranges is for streaming btw, i'll add partial sending later (just for y'all who don't know)
    let mut response = Response::builder()
        .header("Content-Type", content_type)
        .header("Content-Length", metadata.len())
        .header("Accept-Ranges", "bytes")
        .body(body)
        .unwrap();

    add_cors_headers(&mut response);
    response
}

// song search
pub async fn search(State(pool): State<PgPool>, Query(params): Query<HashMap<String, String>>) -> Response<Body> {
    // get raw query
    let search = params.get("q").cloned().unwrap_or_default();

    // build search pattern
    let search = percent_decode_str(&search).decode_utf8_lossy();
    let search_term = search.trim_matches('"').trim();
    let pattern = format!("%{}%", search_term);

    // query for songs
    let query = match sqlx::query(SEARCHSONG)
        .bind(pattern.as_str())
        .fetch_all(&pool).await {
            Ok(q) => q,
            Err(_) => return status_response(StatusCode::INTERNAL_SERVER_ERROR),
        };

    // list of matched songs
    let songs: Vec<Song> = query
        .iter()
        .map(|row| Song {
            id: row.try_get::<i32, _>("id").unwrap_or(0),
            name: row.try_get::<String, _>("name").unwrap_or_default(),
            artist: row.try_get::<String, _>("artist").unwrap_or_default(),
            album: row.try_get::<String, _>("album").unwrap_or_default(),
            duration: row.try_get::<i32, _>("duration").unwrap_or(0),
            filename: row.try_get::<String, _>("filename").unwrap_or_default(),
            cover: None,
        }).collect();

    // serialize and shoot er back
    let json = match serde_json::to_string(&songs) {
        Ok(j) => j,
        Err(_) => return status_response(StatusCode::INTERNAL_SERVER_ERROR),
    };

    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(Body::from(json))
        .unwrap()
}

// cover search
pub async fn cover(Path(id): Path<i64>, State(pool): State<PgPool>) -> Response<Body> {
    // parse album id
    let song_id = id;

    // find cover with that id
    let cover: Option<Vec<u8>> = match sqlx::query_scalar(
        "SELECT a.cover FROM songs s JOIN albums a ON s.album_id = a.id WHERE s.id = $1",
    ).bind(song_id)
    .fetch_one(&pool).await{
        Ok(c) => c,
        Err(_) => return status_response(StatusCode::NOT_FOUND),
    };

    // return that hoe (or error if magic has happened. you prolly deleted a cover)
    let data = match cover {
        Some(d) => d,
        None => return status_response(StatusCode::INTERNAL_SERVER_ERROR),
    };

    Response::builder().body(Body::from(data)).unwrap()
}

// album search
pub async fn album(Path(album_id): Path<i32>, State(pool): State<PgPool>) -> Response<Body> {
    // album id (yoinked from above)
    let album_id = album_id;

    // album metadata
    let row = match sqlx::query(
        "SELECT id, name, artist, cover, runtime, songcount FROM albums WHERE id = $1",
    )
        .bind(album_id)
        .fetch_one(&pool).await {
            Ok(r) => r,
            Err(_) => return status_response(StatusCode::NOT_FOUND),
        };

    // fetch songs
    let songs: Vec<Song> = match sqlx::query_as::<_, Song>(SEARCHALBUM)
        .bind(album_id)
        .fetch_all(&pool).await {
            Ok(s) => s,
            Err(_) => return status_response(StatusCode::INTERNAL_SERVER_ERROR),
        };

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
    let json = match serde_json::to_string(&resp) {
        Ok(j) => j,
        Err(_) => return status_response(StatusCode::INTERNAL_SERVER_ERROR),
    };

    Response::builder().body(Body::from(json)).unwrap()
}
