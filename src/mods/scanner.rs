// backend shit
use anyhow::Result;
use sqlx::{query_file, query_scalar, PgPool};

// filepaths
use std::path::{Path, PathBuf};
use tokio::fs::{create_dir_all, metadata, read_dir, write};

// metadata helpers
use lofty::{
    file::TaggedFileExt,
    picture::Picture,
    prelude::AudioFile,
    probe::Probe,
    tag::{Accessor, Tag},
};

// helper macros to avoid repeating the same Option to String bullshit
macro_rules! tag_str {
    ($tag:expr, $meth:ident, $default:expr) => {
        $tag.and_then(|t| t.$meth().map(|s| s.to_string()))
            .unwrap_or_else(|| $default.to_string())
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
    if query_scalar!("SELECT 1::int FROM songs WHERE filename = $1", filename)
        .fetch_optional(pool)
        .await?
        .is_some()
    {
        return Ok(());
    }

    // open file using a probe, get its tags or the first one (potentially even none) if we don't have it
    let tagged_file = Probe::open(path)?.read()?;
    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());

    // all this shit is data. there's catches for the cases where none is provided as well
    let name: String = tag_str!(tag, title, path.file_stem().unwrap().to_string_lossy());
    let artist: String = tag_str!(tag, artist, "Unknown Artist");
    let album: String = tag_str!(tag, album, "Unknown Album");
    let duration: i32 = tagged_file.properties().duration().as_secs() as i32;
    let cover: Option<String> = match tag.and_then(|t: &Tag| t.pictures().first()) {
        Some(picture) => save_cover_image(&filename, picture).await?,
        None => None,
    };

    // upsert all that info (this shit deals w album and song inserts)
    query_file!(
        "queries/upsert.sql",
        album,
        artist,
        cover,
        name,
        // TODO: make track listings actually work
        0_i32,
        duration,
        filename
    )
    .execute(pool)
    .await?;

    println!("Indexed: {} - {} ({})", artist, name, album);
    Ok(())
}

async fn save_cover_image(filename: &str, picture: &Picture) -> Result<Option<String>> {
    create_dir_all("./static/cover").await?;

    // (i'll do others later but legit 99% of covers are on png or jpg)
    let ext = match picture.mime_type().map(|m| m.as_str()) {
        Some("image/png") => "png",
        _ => "jpg",
    };

    let filename = format!("cover/{}.{}", filename, ext);
    let full_path = Path::new("./static").join(&filename);
    if metadata(&full_path).await.is_err() {
        write(full_path, picture.data()).await?;
    }

    Ok(Some(filename))
}
