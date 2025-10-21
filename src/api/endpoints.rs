// backend related shit
use tokio::fs;
use tokio_util::io::ReaderStream;
use axum::{
    body::Body, middleware::Next,
    extract::{Path, Query, State},
    http::{header, Request as HttpRequest, StatusCode},
    response::{IntoResponse, Response},
};

// dbing
use sqlx::{PgPool, query, query_file_as, query_scalar};

// escaping searches
use percent_encoding::percent_decode_str;
use std::{borrow::Cow, collections::HashMap};
use serde_json::to_string;

// login helpers
use crate::{
    api::{
        cors::add_cors_headers,
        login::verify,
        router::{status_response}
    },
    mods::{
        models::{Album, Song}
    }
};

// auth middleware
pub async fn require_auth(
    State(pool): State<PgPool>,
    req: HttpRequest<Body>,
    next: Next,
) -> Response {
    match verify(&pool, req.headers()).await {
        Ok(_) => next.run(req).await,
        Err(status) => (status, "unauthorized").into_response(),
    }
}

// basic token status check
pub async fn test(
    State(pool): State<PgPool>, 
    req: HttpRequest<Body>
) -> Response<Body> {
    let mut resp: Response<Body>;
    match verify(&pool, req.headers()).await {
        // ok attach a 200
        Ok(_) => {
            resp = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"status":"ok"}"#))
                .unwrap();
        }

        // (left the trailing comma cuz idk if leaving it off is gonna return)
        Err(status) => resp = status_response(status),
    }
    add_cors_headers(&mut resp);
    resp
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
    let body = Body::from_stream(ReaderStream::new(file));

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
    let search: String = params.get("q").cloned().unwrap_or_default();

    // build search pattern
    let search: Cow<'_, str> = percent_decode_str(&search).decode_utf8_lossy();
    let search_term: &str = search.trim_matches('"').trim();
    let pattern: String = format!("%{}%", search_term);

    // query for songs
    let songs: Vec<Song> = match query_file_as!(Song, "queries/searchsong.sql", pattern.as_str())
        .fetch_all(&pool).await {
            Ok(s) => s,
            Err(_) => return status_response(StatusCode::INTERNAL_SERVER_ERROR),
        };

    // serialize and shoot er back
    let json = match to_string(&songs) {
        Ok(j) => j,
        Err(_) => return status_response(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let mut resp = Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json))
        .unwrap();
    add_cors_headers(&mut resp);
    resp
}

// cover search
pub async fn cover(Path(id): Path<i32>, State(pool): State<PgPool>) -> Response<Body> {
    // parse album id
    let song_id: i32 = id;

    // find cover with that id
    let cover: Option<Vec<u8>> = match query_scalar!(
        "SELECT a.cover FROM songs s JOIN albums a ON s.album_id = a.id WHERE s.id = $1",
        song_id
    ).fetch_one(&pool)
    .await {
        Ok(c) => c,
        Err(_) => return status_response(StatusCode::NOT_FOUND),
    };

    // return that hoe (or error if magic has happened. you prolly deleted a cover)
    let data = match cover {
        Some(d) => d,
        None => return status_response(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let mut resp = Response::builder()
        .body(Body::from(data))
        .unwrap();
    add_cors_headers(&mut resp);
    resp
}

// album search
pub async fn album(Path(album_id): Path<i32>, State(pool): State<PgPool>) -> Response<Body> {
    // album id (yoinked from above)
    let album_id: i32 = album_id;

    // album metadata
    let row = match query!(
        "SELECT id, name, artist, cover, runtime, songcount FROM albums WHERE id = $1",
        album_id
    )
    .fetch_one(&pool)
    .await {
        Ok(r) => r,
        Err(_) => return status_response(StatusCode::NOT_FOUND),
    };

    // fetch songs
    let songs: Vec<Song> = match query_file_as!(Song, "queries/searchalbum.sql", album_id)
        .fetch_all(&pool).await {
            Ok(s) => s,
            Err(_) => return status_response(StatusCode::INTERNAL_SERVER_ERROR),
        };

    // build into one big fat album
    let resp = Album {
        id: row.id,
        name: row.name,
        artist: row.artist,
        runtime: row.runtime,
        songcount: row.songcount,
        songs,
    };

    // serialize and send
    let json = match to_string(&resp) {
        Ok(j) => j,
        Err(_) => return status_response(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let mut resp = Response::builder()
        .body(Body::from(json))
        .unwrap();
    add_cors_headers(&mut resp);
    resp
}
