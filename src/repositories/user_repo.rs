use crate::errors::Result;
use crate::models::User;
use sqlx::PgPool;

pub async fn find_by_id(pool: &PgPool, user_id: &str) -> Result<Option<User>> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(user)
}

pub async fn find_or_create(pool: &PgPool, user_id: &str) -> Result<User> {
    sqlx::query("INSERT INTO users (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING")
        .bind(user_id)
        .execute(pool)
        .await?;
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    Ok(user)
}

pub async fn update_privacy(pool: &PgPool, user_id: &str, is_private: bool) -> Result<()> {
    sqlx::query("UPDATE users SET is_private = $2, updated_at = NOW() WHERE user_id = $1")
        .bind(user_id)
        .bind(is_private)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn add_voice_time(pool: &PgPool, user_id: &str, duration_ms: i64) -> Result<()> {
    sqlx::query("UPDATE users SET total_voice_time = total_voice_time + $2, last_seen = NOW(), updated_at = NOW() WHERE user_id = $1")
        .bind(user_id)
        .bind(duration_ms)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn recompute_voice_time(pool: &PgPool, user_id: &str) -> Result<()> {
    sqlx::query(
        "UPDATE users u SET total_voice_time = COALESCE((SELECT SUM(vs.duration) FROM voice_sessions vs WHERE vs.user_id = u.user_id), 0), last_seen = NOW(), updated_at = NOW() WHERE u.user_id = $1"
    )
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn recompute_all_voice_times(pool: &PgPool) -> Result<()> {
    sqlx::query(
        "UPDATE users u SET total_voice_time = COALESCE((SELECT SUM(vs.duration) FROM voice_sessions vs WHERE vs.user_id = u.user_id), 0), updated_at = NOW()"
    )
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_username_history(
    pool: &PgPool,
    user_id: &str,
    history: serde_json::Value,
) -> Result<()> {
    sqlx::query("UPDATE users SET username_history = $2, updated_at = NOW() WHERE user_id = $1")
        .bind(user_id)
        .bind(history)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_avatar_history(
    pool: &PgPool,
    user_id: &str,
    history: serde_json::Value,
) -> Result<()> {
    sqlx::query("UPDATE users SET avatar_history = $2, updated_at = NOW() WHERE user_id = $1")
        .bind(user_id)
        .bind(history)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_nickname_history(
    pool: &PgPool,
    user_id: &str,
    history: serde_json::Value,
) -> Result<()> {
    sqlx::query("UPDATE users SET nickname_history = $2, updated_at = NOW() WHERE user_id = $1")
        .bind(user_id)
        .bind(history)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn reset_user(pool: &PgPool, user_id: &str) -> Result<()> {
    sqlx::query(
        "UPDATE users SET is_private = FALSE, total_voice_time = 0, premium = FALSE, username_history = '[]'::jsonb, avatar_history = '[]'::jsonb, nickname_history = '[]'::jsonb, last_seen = NULL, updated_at = NOW() WHERE user_id = $1"
    )
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_voice_ranking(
    pool: &PgPool,
    guild_id: &str,
    limit: i64,
) -> Result<Vec<(String, i64)>> {
    let rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT vs.user_id, COALESCE(SUM(vs.duration), 0)::bigint as total_duration FROM voice_sessions vs WHERE vs.guild_id = $1 GROUP BY vs.user_id ORDER BY total_duration DESC LIMIT $2"
    )
        .bind(guild_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn clear_sessions(pool: &PgPool, user_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM voice_sessions WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}
