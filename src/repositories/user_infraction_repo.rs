use crate::errors::Result;
use sqlx::PgPool;

/// Increments a user's infraction count for a specific guild.
/// If the last infraction occurred more than 7 days ago, the count is reset to 1.
/// Returns the updated infraction count.
pub async fn register_infraction(pool: &PgPool, guild_id: &str, user_id: &str) -> Result<i32> {
    let row: (i32,) = sqlx::query_as(
        "INSERT INTO user_infractions (guild_id, user_id, infraction_count, last_infraction_at) \
         VALUES ($1, $2, 1, NOW()) \
         ON CONFLICT (guild_id, user_id) \
         DO UPDATE SET \
           infraction_count = CASE WHEN NOW() - user_infractions.last_infraction_at > INTERVAL '7 days' THEN 1 ELSE user_infractions.infraction_count + 1 END, \
           last_infraction_at = NOW() \
         RETURNING infraction_count"
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}
