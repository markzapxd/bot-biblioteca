use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    Ok(pool)
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(74204003)").execute(&mut *tx).await?;
    sqlx::raw_sql("CREATE TABLE IF NOT EXISTS bot_schema_migrations (version INT PRIMARY KEY, applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW())").execute(&mut *tx).await?;
    for (version, sql) in [
        (1, include_str!("../../migrations/001_initial_schema.sql")),
        (2, include_str!("../../migrations/002_custom_cards.sql")),
        (3, include_str!("../../migrations/003_voice_integrity.sql")),
        (4, include_str!("../../migrations/004_support_workflows.sql")),
        (5, include_str!("../../migrations/005_lockdown_snapshots.sql")),
    ] {
        let applied: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bot_schema_migrations WHERE version = $1)").bind(version).fetch_one(&mut *tx).await?;
        if !applied {
            sqlx::raw_sql(sql).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO bot_schema_migrations(version) VALUES ($1)").bind(version).execute(&mut *tx).await?;
        }
    }
    tx.commit().await?;
    Ok(())
}
