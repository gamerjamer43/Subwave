// backend related shit
use hyper::{Body, Request, Response, StatusCode, Method, header};
use sqlx::{SqlitePool, Row};
use percent_encoding::percent_decode_str;

// da song model
use crate::models::{Song, Album};
use crate::login::{signup, login, AuthRequest};

// basic handler
pub async fn handle(req: Request<Body>, pool: SqlitePool) -> Result<Response<Body>, StatusCode> {
    let path = req.uri().path();

    // POST routes for authentication ONLY
    match (req.method(), path) {
        (&Method::POST, "/api/signup") => {
            let body_bytes = hyper::body::to_bytes(req.into_body())
                .await
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            
            let auth_req: AuthRequest = serde_json::from_slice(&body_bytes)
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            
            return signup(pool, auth_req).await;
        }

        (&Method::POST, "/api/login") => {
            let body_bytes = hyper::body::to_bytes(req.into_body())
                .await
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            
            let auth_req: AuthRequest = serde_json::from_slice(&body_bytes)
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            
            return login(pool, auth_req).await;
        }
        _ => {}
    }

    // we only do get methods otherwise
    if req.method() != Method::GET {
        return Err(StatusCode::METHOD_NOT_ALLOWED);
    }

    // match path properly
    match path {
        s if s.starts_with("/api/search") => return search(req, pool).await,
        c if c.starts_with("/api/cover") => return cover(path, pool).await,
        a if a.starts_with("/api/album") => album(path, pool).await,
        _ => Err(StatusCode::NOT_FOUND),
    }
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
    // parse song name
    let id: i64 = path.trim_start_matches("/api/cover/")
        .parse()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    // search for one cover: albums store the cover, join via album_id
    let row = sqlx::query("SELECT a.cover FROM songs s JOIN albums a ON s.album_id = a.id WHERE s.id = ?")
        .bind(id)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // get that cover
    let cover: Option<Vec<u8>> = row.get("cover");
    if let Some(data) = cover {
        return Ok(Response::builder()
            .header(header::CONTENT_TYPE, "image/jpeg")
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
            .body(Body::from(data))
            .unwrap());
    } 
    
    // proper error handling
    else {
        return Err(StatusCode::NOT_FOUND);
    }
}

// album search
async fn album(path: &str, pool: SqlitePool) -> Result<Response<Body>, StatusCode> {
    // album id (yoinked frm above)
    let album_id: u16 = path.trim_start_matches("/api/album/")
        .parse()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // album metadata
    let row = sqlx::query("SELECT id, name, artist, cover, runtime, songcount FROM albums WHERE id = ?")
        .bind(album_id)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let album_name: String = row.get("name");
    let album_artist: String = row.get("artist");
    let album_runtime: u16 = row.get("runtime");
    let album_songcount: u8 = row.get("songcount");

    // fetch all songs in said album. this ones getting fat so i may move it
    let rows = sqlx::query("SELECT s.id, s.name, s.duration, s.filename, a.name as album, a.artist as artist FROM songs s JOIN albums a ON s.album_id = a.id WHERE a.id = ? ORDER BY s.track_number ASC")
        .bind(album_id)
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let songs: Vec<Song> = rows.iter().map(|row| {
        // fat blob of info
        let id: u16 = row.get("id");
        let name: String = row.get("name");
        let artist: String = row.get("artist");
        let album: String = row.get("album");
        let duration: u16 = row.get("duration");
        let filename: String = row.get("filename");

        // thanks serde!!!!
        Song {
            id,
            name,
            artist,
            album,
            cover: None,
            duration: duration,
            filename,
        }
    }).collect();

    // build a response w the info
    let resp = Album {
        id: album_id,
        name: album_name,
        artist: album_artist,
        runtime: album_runtime,
        songcount: album_songcount,
        songs,
    };

    let json = serde_json::to_string(&resp).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(Body::from(json))
        .unwrap())
}