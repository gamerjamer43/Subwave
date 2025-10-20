// backend shit
use sqlx::PgPool;
use anyhow::Result;

// filepaths
use tokio::fs::read_dir;
use std::path::{Path, PathBuf};

// metadata helpers
use lofty::{
    file::TaggedFileExt,
    prelude::AudioFile, 
    probe::Probe, 
    tag::Accessor
};

// i'm saving one line (and sanity)
const UPSERTSONG: &str = include_str!("queries/upsertsong.sql");
const UPSERTALBUM: &str = include_str!("queries/upsertalbum.sql");

// helper macros to avoid repeating the same Option to String bullshit
macro_rules! tag_str {
    ($tag:expr, $meth:ident, $default:expr) => {
        $tag.and_then(|t| t.$meth().map(|s| s.to_string()))
            .unwrap_or_else(|| $default.to_string())
    };
}

macro_rules! tag_opt_pic {
    ($tag:expr) => {
        $tag.and_then(|t| t.pictures().first().map(|p| p.data().to_vec()))
    };
}

// scan music folder and extract metadata
pub async fn scan(pool: &PgPool, folder: &str) -> Result<()> {
    let mut entries = read_dir(folder).await?;

    // go thru each dir entry
    while let Some(entry) = entries.next_entry().await? {
        let path: PathBuf = entry.path();

        // skip if no extension
        let ext: String = match path.extension() {
            Some(e) => e.to_string_lossy().to_lowercase(),
            None => continue,
        };

        // jettison unsupported extensions (mp3 flac ogg and wav frn)
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
async fn index(pool: &PgPool, path: &Path) -> Result<()> {
    // build the filename from stored
    let filename: String = format!("{}", path.file_name().unwrap().to_string_lossy());

    // skip reindexing
    if let Some(_) = sqlx::query("SELECT 1 FROM songs WHERE filename = $1")
        .bind(&filename)
        .fetch_optional(pool)
        .await? {
            return Ok(());
        }

    // open file using a probe, get its tags or the first one (potentially even none) if we don't have it
    let tagged_file = Probe::open(path)?.read()?;
    let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());

    // all this shit is data. there's catches for the cases where none is provided as well
    let name: String = tag_str!(tag, title, path.file_stem().unwrap().to_string_lossy());
    let artist: String = tag_str!(tag, artist, "Unknown Artist");
    let album: String = tag_str!(tag, album, "Unknown Album");
    let cover: Option<Vec<u8>> = tag_opt_pic!(tag);
    let duration: i32 = tagged_file.properties().duration().as_secs() as i32;

    // this is the LAST place to save lines and time. this could all be one query.
    // create albums
    sqlx::query(UPSERTALBUM)
        .bind(&album)
        .bind(&artist)
        .bind(cover.as_deref())
        .execute(pool).await?;

    // album ids
    let album_id: i32 = sqlx::query_scalar(
        "SELECT id FROM albums WHERE name = $1 AND artist = $2"
    )
        .bind(&album)
        .bind(&artist)
        .fetch_one(pool).await?;

    // ts so fucked. track number is a placeholder zero if not found
    sqlx::query(UPSERTSONG)
        .bind(&name)
        .bind(album_id)
        .bind(0_i32) // placeholder track number when none is available
        .bind(duration)
        .bind(&filename)
        .execute(pool).await?;

    // update incrementally
    sqlx::query(
        "UPDATE albums SET songcount = songcount + 1, runtime = runtime + $1 WHERE id = $2;"
    )
        .bind(duration).bind(album_id)
        .execute(pool).await?;

    println!("Indexed: {} - {} ({})", artist, name, album);
    Ok(())
}