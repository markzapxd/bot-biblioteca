use crate::errors::Result;
use crate::models::VoiceSession;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

/// Serialize transitions in PostgreSQL, including when no active row exists yet.
/// The user row also serializes aggregate updates across guilds.
pub async fn transition(
    pool: &PgPool,
    user_id: &str,
    guild_id: &str,
    guild_name: Option<&str>,
    destination: Option<(&str, &str)>,
    now: DateTime<Utc>,
) -> Result<Option<VoiceSession>> {
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO users(user_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT user_id FROM users WHERE user_id=$1 FOR UPDATE")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    let active = sqlx::query_as::<_, VoiceSession>(
        "SELECT * FROM voice_sessions WHERE user_id=$1 AND guild_id=$2 AND active FOR UPDATE",
    )
    .bind(user_id)
    .bind(guild_id)
    .fetch_optional(&mut *tx)
    .await?;
    if active.as_ref().map(|s| s.channel_id.as_str()) == destination.map(|d| d.0) {
        tx.commit().await?;
        return Ok(active);
    }
    if let Some(session) = active {
        sqlx::query("UPDATE voice_sessions SET active=FALSE, left_at=GREATEST(joined_at,$2), duration=GREATEST(0,(EXTRACT(EPOCH FROM ($2-joined_at))*1000)::bigint), updated_at=$2 WHERE id=$1")
            .bind(session.id).bind(now).execute(&mut *tx).await?;
    }
    let session = if let Some((channel_id, channel_name)) = destination {
        Some(sqlx::query_as::<_, VoiceSession>("INSERT INTO voice_sessions(user_id,guild_id,guild_name,channel_id,channel_name,joined_at) VALUES ($1,$2,$3,$4,$5,$6) RETURNING *")
            .bind(user_id).bind(guild_id).bind(guild_name).bind(channel_id).bind(channel_name).bind(now).fetch_one(&mut *tx).await?)
    } else {
        None
    };
    sqlx::query("UPDATE users SET total_voice_time=COALESCE((SELECT SUM(duration) FROM voice_sessions WHERE user_id=$1),0), last_seen=$2, updated_at=$2 WHERE user_id=$1")
        .bind(user_id).bind(now).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(session)
}

pub async fn create(
    pool: &PgPool,
    user_id: &str,
    guild_id: &str,
    guild_name: Option<&str>,
    channel_id: &str,
    channel_name: &str,
    joined_at: DateTime<Utc>,
) -> Result<VoiceSession> {
    let session = sqlx::query_as::<_, VoiceSession>(
        "INSERT INTO voice_sessions (user_id, guild_id, guild_name, channel_id, channel_name, joined_at) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *"
    )
        .bind(user_id)
        .bind(guild_id)
        .bind(guild_name)
        .bind(channel_id)
        .bind(channel_name)
        .bind(joined_at)
        .fetch_one(pool)
        .await?;
    Ok(session)
}

pub async fn close(
    pool: &PgPool,
    id: i32,
    left_at: DateTime<Utc>,
    duration_ms: i64,
    members_at_end: serde_json::Value,
) -> Result<()> {
    sqlx::query(
        "UPDATE voice_sessions SET left_at = $2, duration = $3, members_at_end = $4, active = FALSE, updated_at = NOW() WHERE id = $1"
    )
        .bind(id)
        .bind(left_at)
        .bind(duration_ms)
        .bind(members_at_end)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete(pool: &PgPool, id: i32) -> Result<()> {
    sqlx::query("DELETE FROM voice_sessions WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn find_active_by_user_guild(
    pool: &PgPool,
    user_id: &str,
    guild_id: &str,
) -> Result<Option<VoiceSession>> {
    let session = sqlx::query_as::<_, VoiceSession>(
        "SELECT * FROM voice_sessions WHERE user_id = $1 AND guild_id = $2 AND active = TRUE",
    )
    .bind(user_id)
    .bind(guild_id)
    .fetch_optional(pool)
    .await?;
    Ok(session)
}

pub async fn find_all_active(pool: &PgPool) -> Result<Vec<VoiceSession>> {
    let sessions =
        sqlx::query_as::<_, VoiceSession>("SELECT * FROM voice_sessions WHERE active = TRUE")
            .fetch_all(pool)
            .await?;
    Ok(sessions)
}

pub async fn find_by_user(pool: &PgPool, user_id: &str, limit: i64) -> Result<Vec<VoiceSession>> {
    let sessions = sqlx::query_as::<_, VoiceSession>(
        "SELECT * FROM voice_sessions WHERE user_id = $1 ORDER BY joined_at DESC LIMIT $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(sessions)
}

pub async fn get_user_stats(pool: &PgPool, user_id: &str) -> Result<(i64, i64)> {
    let row = sqlx::query_as::<_, (i64, i64)>(
        "SELECT COUNT(*)::bigint, COALESCE(SUM(duration), 0)::bigint FROM voice_sessions WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn find_last_by_user(pool: &PgPool, user_id: &str) -> Result<Option<VoiceSession>> {
    let session = sqlx::query_as::<_, VoiceSession>(
        "SELECT * FROM voice_sessions WHERE user_id = $1 ORDER BY joined_at DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(session)
}

pub async fn update_active_durations(pool: &PgPool, now: DateTime<Utc>) -> Result<()> {
    sqlx::query(
        "UPDATE voice_sessions SET duration = EXTRACT(EPOCH FROM ($1 - joined_at)) * 1000, updated_at = NOW() WHERE active = TRUE"
    )
        .bind(now)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn close_all_active(pool: &PgPool, _now: DateTime<Utc>) -> Result<Vec<VoiceSession>> {
    let sessions = sqlx::query_as::<_, VoiceSession>(
        "UPDATE voice_sessions SET left_at = updated_at, duration = EXTRACT(EPOCH FROM (updated_at - joined_at)) * 1000, active = FALSE WHERE active = TRUE RETURNING *"
    )
        .fetch_all(pool)
        .await?;
    Ok(sessions)
}

pub async fn find_by_guild_with_duration(
    pool: &PgPool,
    guild_id: &str,
    limit: i64,
) -> Result<Vec<VoiceSession>> {
    let sessions = sqlx::query_as::<_, VoiceSession>(
        "SELECT * FROM voice_sessions WHERE guild_id = $1 ORDER BY duration DESC LIMIT $2",
    )
    .bind(guild_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(sessions)
}

pub async fn find_by_user_guild(
    pool: &PgPool,
    user_id: &str,
    guild_id: &str,
    limit: i64,
) -> Result<Vec<VoiceSession>> {
    Ok(sqlx::query_as::<_, VoiceSession>("SELECT * FROM voice_sessions WHERE user_id = $1 AND guild_id = $2 ORDER BY joined_at DESC LIMIT $3")
        .bind(user_id).bind(guild_id).bind(limit).fetch_all(pool).await?)
}
