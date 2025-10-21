use anyhow::Result;
use sqlx::{query, PgPool};

// initialize database schema
pub async fn init(pool: &PgPool) -> Result<()> {
    // ig we're doing this compile time so sqlx stops WHINING
    let sql = include_str!("../queries/createdb.sql");

    for statement in sql.split_terminator(';') {
        let statement = statement.trim();
        if statement.is_empty() {
            continue;
        }

        query(statement).execute(pool).await?;
    }

    Ok(())
}