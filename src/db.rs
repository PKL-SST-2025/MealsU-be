use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing::info;

pub async fn create_pool(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    info!("Connected to Postgres");

    // --- Schema Migration --- 

    // 1. Create users table if it doesn't exist (for first-time setup)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY,
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT now()
        );
        "#,
    )
    .execute(&pool)
    .await?;

    // 2. Add profile columns to the users table if they don't exist (for existing users)
    let columns = [
        ("name", "TEXT"),
        ("dietary_preference", "TEXT"),
        ("gender", "TEXT"),
        ("age", "INTEGER"),
        ("bio", "TEXT"),
        ("avatar", "TEXT"),
    ];

    for (col_name, col_type) in columns.iter() {
        let query_str = format!("ALTER TABLE users ADD COLUMN IF NOT EXISTS {} {}", col_name, col_type);
        sqlx::query(&query_str).execute(&pool).await?;
    }

    // 3. Create user_measurements table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_measurements (
            user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
            height REAL,
            current_weight REAL,
            target_weight REAL,
            waist REAL,
            chest REAL,
            thigh REAL,
            arm REAL,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
        );
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}
