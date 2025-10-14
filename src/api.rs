// backend related shit
use hyper::{Body, Request, Response, StatusCode, Method, header};
use sqlx::{SqlitePool, Row};
use percent_encoding::percent_decode_str;

// da song model
use crate::models::Song;
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
    let rows = sqlx::query_file!("queries/searchsong.sql", pattern, pattern, pattern)
     .fetch_all(&pool)
     .await
     .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // returned songs go here
    let songs: Vec<Song> = rows.iter().map(|row| Song {
        id: row.id,
        name: row.name.clone(),
        artist: row.artist.clone(),
        album: row.album.clone(),
        cover: None, // don't send cover in list view
        duration: row.duration as i16,
        filename: row.filename.clone(),
    }).collect();
    
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