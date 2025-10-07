use anyhow::Result;
use sqlx::SqlitePool;

// initialize database schema
pub async fn init(pool: &SqlitePool) -> Result<()> {
    // create table if it doesn't already exist
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS songs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            artist TEXT NOT NULL,
            album TEXT NOT NULL,
            cover BLOB,
            duration INTEGER NOT NULL,
            filename TEXT NOT NULL UNIQUE
        )
        "#,
    ).execute(pool)
     .await?;
    
    // ok!!!!
    Ok(())
}