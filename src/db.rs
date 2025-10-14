use anyhow::Result;
use sqlx::SqlitePool;

// initialize database schema
pub async fn init(pool: &SqlitePool) -> Result<()> {
    // ig we're doing this compile time so sqlx stops WHINING
    let sql = include_str!("../queries/createdb.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}