use std::path::Path;
use anyhow::Result;
use sqlx::SqlitePool;
use tokio::fs;
use lofty::{prelude::AudioFile, probe::Probe, tag::Accessor, file::TaggedFileExt};

// helper macros to avoid repeating the same Option to String bullshit
macro_rules! tag_str {
    ($tag:expr, $meth:ident, $default:expr) => {
        $tag
            .and_then(|t| t.$meth().map(|s| s.to_string()))
            .unwrap_or_else(|| $default.to_string())
    };
}

macro_rules! tag_opt_pic {
    ($tag:expr) => {
        $tag.and_then(|t| t.pictures().first().map(|p| p.data().to_vec()))
    };
}

// scan music folder and extract metadata
pub async fn scan(pool: &SqlitePool, folder: &str) -> Result<()> {
    let mut entries = fs::read_dir(folder).await?;
    
    // go thru each dir entry
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        // skip if no extension
        let ext = match path.extension() {
            Some(e) => e.to_string_lossy().to_lowercase(),
            None => continue,
        };

        // jettison unsupported extensions
        if !matches!(ext.as_str(), "mp3" | "flac" | "ogg" | "wav") {
            continue;
        }

        // now we try indexing
        if let Err(e) = index(pool, &path).await {
            eprintln!("Error indexing {:?}: {}", path, e);
        }
    }
    
    // ok!
    Ok(())
}

// index helper for dbing
async fn index(pool: &SqlitePool, path: &Path) -> Result<()> {
    // build the filename from stored
    let filename = format!("{}", path.file_name().unwrap().to_string_lossy());

    // skip reindexing
    if let Some(_) = sqlx::query("SELECT 1 FROM songs WHERE filename = ?")
     .bind(&filename)
     .fetch_optional(pool).await? {
        println!("Skipping already indexed: {}", filename);
        return Ok(());
    }

    // open file using a probe, get its tags or the first one (potentially even none) if we don't have it
    let tagged_file = Probe::open(path)?.read()?;
    let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());

    // all this shit is data. there's catches for the cases where none is provided as well
    let name = tag_str!(tag, title, path.file_stem().unwrap().to_string_lossy().to_string());
    let artist = tag_str!(tag, artist, "Unknown Artist");
    let album = tag_str!(tag, album, "Unknown Album");
    let cover = tag_opt_pic!(tag);
    let duration = tagged_file.properties().duration().as_secs() as i64;
    
    // create albums
    sqlx::query(
        "INSERT OR IGNORE INTO albums (name, artist, cover, runtime, songcount)
        VALUES (?, ?, ?, 0, 0);"
    )
    .bind(&album).bind(&artist).bind(&cover)
    .execute(pool).await?;

    // album ids
    let album_id: i64 = sqlx::query_scalar(
        "SELECT id FROM albums WHERE name = ? AND artist = ?;"
    )
    .bind(&album).bind(&artist)
    .fetch_one(pool).await?;

    // ts so fucked. track number is a placeholder zero if not found
    sqlx::query(
        "INSERT OR REPLACE INTO songs (name, album_id, track_number, duration, filename)
        VALUES (?, ?, ?, ?, ?);"
    )
    .bind(&name).bind(album_id).bind(0_i32) // placeholder track number when none is available
    .bind(duration).bind(&filename)
    .execute(pool).await?;
    
    // update incrementally
    sqlx::query("UPDATE albums SET songcount = songcount + 1, runtime = runtime + ? WHERE id = ?;")
        .bind(duration)
        .bind(album_id)
        .execute(pool)
        .await?;

    println!("Indexed: {} - {} ({})", artist, name, album);
    Ok(())
}