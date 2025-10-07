use hyper::{Body, Request, Response, StatusCode, Method, header};
use sqlx::{SqlitePool, Row};
use percent_encoding::percent_decode_str;
use crate::models::Song;

// basic handler
pub async fn handle(req: Request<Body>, pool: SqlitePool) -> Result<Response<Body>, StatusCode> {
    if req.method() != Method::GET {
        return Err(StatusCode::METHOD_NOT_ALLOWED);
    }
    
    let path = req.uri().path();
    
    if path == "/api/search" {
        return search(req, pool).await;
    }
    
    if path.starts_with("/api/cover/") {
        return cover(path, pool).await;
    }
    
    Err(StatusCode::NOT_FOUND)
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
    let search_pattern = format!("%{}%", search_term);
    let rows = sqlx::query(
        "SELECT id, name, artist, album, duration, filename FROM songs 
         WHERE name LIKE ? OR artist LIKE ? OR album LIKE ?
         LIMIT 50"
    ).bind(&search_pattern)
     .bind(&search_pattern)
     .bind(&search_pattern)
     .fetch_all(&pool)
     .await
     .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // returned songs go here
    let songs: Vec<Song> = rows.iter().map(|row| Song {
        id: row.get("id"),
        name: row.get("name"),
        artist: row.get("artist"),
        album: row.get("album"),
        cover: None, // don't send cover in list view
        duration: row.get("duration"),
        filename: row.get("filename"),
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
    
    // search for one cover
    let row = sqlx::query("SELECT cover FROM songs WHERE id = ?")
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
    } else {
        return Err(StatusCode::NOT_FOUND);
    }
}