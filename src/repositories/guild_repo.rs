use crate::errors::Result;
use crate::models::Guild;
use sqlx::PgPool;

pub async fn find_by_id(pool: &PgPool, guild_id: &str) -> Result<Option<Guild>> {
    let guild = sqlx::query_as::<_, Guild>("SELECT * FROM guilds WHERE guild_id = $1")
        .bind(guild_id)
        .fetch_optional(pool)
        .await?;
    Ok(guild)
}

pub async fn upsert(pool: &PgPool, guild_id: &str) -> Result<Guild> {
    let guild = sqlx::query_as::<_, Guild>(
        "INSERT INTO guilds (guild_id) VALUES ($1) ON CONFLICT (guild_id) DO UPDATE SET updated_at = NOW() RETURNING *"
    )
        .bind(guild_id)
        .fetch_one(pool)
        .await?;
    Ok(guild)
}

pub async fn update_field(
    pool: &PgPool,
    guild_id: &str,
    field_name: &str,
    value: &str,
) -> Result<()> {
    let allowed = [
        "prefix",
        "member_role_id",
        "staff_channel_id",
        "welcome_channel_id",
        "log_channel_id",
        "admin_role_id",
        "staff_role_id",
        "ticket_category_id",
        "frin_monitor_channel_id",
        "webhook_url",
        "premium",
        "track_mute",
        "track_deaf",
    ];
    if !allowed.contains(&field_name) {
        return Err(crate::errors::BotError::Validation(format!(
            "Invalid field: {}",
            field_name
        )));
    }
    let query = format!(
        "UPDATE guilds SET {} = $2, updated_at = NOW() WHERE guild_id = $1",
        field_name
    );
    sqlx::query(&query)
        .bind(guild_id)
        .bind(value)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_modules(
    pool: &PgPool,
    guild_id: &str,
    modules: serde_json::Value,
) -> Result<()> {
    sqlx::query("UPDATE guilds SET modules = $2, updated_at = NOW() WHERE guild_id = $1")
        .bind(guild_id)
        .bind(modules)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn find_all(pool: &PgPool) -> Result<Vec<Guild>> {
    let guilds = sqlx::query_as::<_, Guild>("SELECT * FROM guilds")
        .fetch_all(pool)
        .await?;
    Ok(guilds)
}
