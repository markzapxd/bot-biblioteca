use crate::errors::Result;
use crate::models::User;
use sqlx::PgPool;

#[derive(Clone, Copy)]
pub enum HistoryKind { Username, Avatar, Nickname }

/// Atomic row-locked append. Deduplicate adjacent equal values (per guild for
/// nicknames), ignoring timestamps, then keep the latest 500 appended entries.
pub async fn append_history(pool: &PgPool, user_id: &str, kind: HistoryKind, entry: serde_json::Value) -> Result<()> {
    let (column, key) = match kind {
        HistoryKind::Username => ("username_history", "name"),
        HistoryKind::Avatar => ("avatar_history", "url"),
        HistoryKind::Nickname => ("nickname_history", "name"),
    };
    let sql = format!("INSERT INTO users(user_id, {column}) VALUES ($1, jsonb_build_array($2::jsonb)) ON CONFLICT(user_id) DO UPDATE SET {column} = (SELECT COALESCE(jsonb_agg(value ORDER BY ord), '[]'::jsonb) FROM jsonb_array_elements(COALESCE(users.{column}, '[]'::jsonb) || jsonb_build_array($2::jsonb)) WITH ORDINALITY e(value,ord) WHERE ord > jsonb_array_length(COALESCE(users.{column}, '[]'::jsonb)) + 1 - 500), updated_at=NOW() WHERE (SELECT value->>$3 FROM jsonb_array_elements(COALESCE(users.{column}, '[]'::jsonb)) WITH ORDINALITY e(value,ord) WHERE ($4::bool = FALSE OR value->>'guild_id' IS NOT DISTINCT FROM $2::jsonb->>'guild_id') ORDER BY ord DESC LIMIT 1) IS DISTINCT FROM $2::jsonb->>$3");
    sqlx::query(&sql).bind(user_id).bind(entry).bind(key).bind(matches!(kind, HistoryKind::Nickname | HistoryKind::Avatar)).execute(pool).await?;
    Ok(())
}

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
    let mut tx = pool.begin().await?;
    lock_voice_user(&mut tx, user_id).await?;
    sqlx::query("UPDATE users SET total_voice_time = COALESCE(total_voice_time, 0) + $2, last_seen = NOW(), updated_at = NOW() WHERE user_id = $1")
        .bind(user_id)
        .bind(duration_ms)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

async fn lock_voice_user(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, user_id: &str) -> Result<()> {
    sqlx::query("INSERT INTO users(user_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(user_id).execute(&mut **tx).await?;
    sqlx::query("SELECT user_id FROM users WHERE user_id=$1 FOR UPDATE")
        .bind(user_id).execute(&mut **tx).await?;
    Ok(())
}

pub async fn recompute_voice_time(pool: &PgPool, user_id: &str) -> Result<()> {
    let mut tx = pool.begin().await?;
    // Separate statements are essential at READ COMMITTED: SUM must get its
    // snapshot after any transition holding the user row has committed.
    lock_voice_user(&mut tx, user_id).await?;
    sqlx::query(
        "UPDATE users u SET total_voice_time = COALESCE((SELECT SUM(vs.duration) FROM voice_sessions vs WHERE vs.user_id = u.user_id), 0), last_seen = NOW(), updated_at = NOW() WHERE u.user_id = $1"
    )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn recompute_all_voice_times(pool: &PgPool) -> Result<()> {
    let users = sqlx::query_scalar::<_, String>("SELECT user_id FROM users ORDER BY user_id")
        .fetch_all(pool).await?;
    for user in users {
        recompute_voice_time(pool, &user).await?;
    }
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
