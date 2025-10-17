use anyhow::Result;
use sqlx::PgPool;

// initialize database schema
pub async fn init(pool: &PgPool) -> Result<()> {
    // ig we're doing this compile time so sqlx stops WHINING
    let sql = include_str!("../queries/createdb.sql");

    for statement in sql.split_terminator(';') {
        let statement = statement.trim();
        if statement.is_empty() {continue;}

        sqlx::query(statement).execute(pool).await?;
    }

    Ok(())
}